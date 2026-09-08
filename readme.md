# Code Execution Server

The HTTP server for a distributed code execution platform, built with **Rust and Axum**.

The server is responsible for accepting code execution requests, creating and persisting jobs, publishing jobs to Kafka, and providing APIs for querying execution status and results.

Code execution itself is handled by independent Workers, allowing the API layer and execution layer to scale separately.

## ✨ Features

* 🚀 Asynchronous HTTP API with **Axum + Tokio**
* 📝 Create and manage code execution jobs
* 📬 Publish jobs to **Kafka**
* 💾 Persistent job state with **PostgreSQL**
* 🔒 Atomic job state transitions and locking
* 🔍 Query job status and execution results
* 🛑 Job cancellation support
* 📊 Prometheus metrics
* 📝 Structured logging with `tracing`
* 🐳 Docker Compose development environment

## 🏗️ Architecture

The Server acts as the entry point of the code execution system.

```text
                     ┌───────────────┐
                     │     Client    │
                     └───────┬───────┘
                             │
                             │ HTTP
                             ▼
                    ┌──────────────────┐
                    │      Server      │
                    │                  │
                    │      Axum        │
                    │      Tokio        │
                    └───────┬──────────┘
                            │
                 ┌──────────┴──────────┐
                 │                     │
                 ▼                     ▼
        ┌────────────────┐    ┌────────────────┐
        │   PostgreSQL   │    │     Kafka      │
        │                │    │                │
        │  Job State     │    │   Job Queue    │
        └────────────────┘    └───────┬────────┘
                                      │
                                      ▼
                              ┌───────────────┐
                              │    Workers    │
                              │               │
                              │ Docker Runner │
                              └───────────────┘
```

The Server does **not** execute user code directly.

Instead, it creates a Job and publishes a message to Kafka:

```text
HTTP Request
     │
     ▼
Create Job
     │
     ├──────────────► PostgreSQL
     │
     ▼
Kafka Job Message
     │
     ▼
Worker
     │
     ▼
Docker Sandbox
```

## 🔄 Job Lifecycle

A job is represented by a state machine:

```text
                  ┌───────────┐
                  │  Queued   │
                  └─────┬─────┘
                        │
                        ▼
                  ┌───────────┐
                  │  Running  │
                  └─────┬─────┘
                        │
              ┌─────────┼─────────┐
              ▼         ▼         ▼
        ┌──────────┐ ┌────────┐ ┌────────────────────┐
        │Completed │ │ Failed │ │ TimeLimitExceeded │
        └──────────┘ └────────┘ └────────────────────┘
```

The Server persists job state in PostgreSQL, while Workers are responsible for executing the actual job.

## 📡 API

### Create a Job

```http
POST /runs
Content-Type: application/json
```

Example request:

```json
{ç
  "code": "fn main() { println!(\"Hello, Rust!\"); }"
}
```

The Server:

1. Creates a new Job ID.
2. Persists the job in PostgreSQL.
3. Publishes a `JobMessage` to Kafka.
4. Returns the Job ID to the client.

Example response:

```json
{
  "job_id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
}
```

### Get Job

```http
GET /runs/:id
```

Example response:

```json
{
  "id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
  "status": "completed",
  "stdout": "Hello, Rust!\n",
  "stderr": "",
  "exit_code": 0
}
```

### Job Cancellation

```http
POST /runs/:id/cancel
```

The cancellation request updates the job state and allows the execution layer to stop a running job.

## 📬 Kafka

Kafka is used as the asynchronous boundary between the Server and Workers.

```text
┌──────────────┐
│    Server    │
└──────┬───────┘
       │
       │ JobMessage
       ▼
┌──────────────┐
│    Kafka     │
└──────┬───────┘
       │
       │ consume
       ▼
┌──────────────┐
│    Worker    │
└──────────────┘
```

A typical job message contains the information required by a Worker to process the job:

```text
JobMessage
├── job_id
└── execution information
```

This design keeps the HTTP request path independent from the execution time of user code.

For example, a compilation that takes several seconds does not block the HTTP request until execution finishes.

## 💾 PostgreSQL

PostgreSQL stores persistent execution state.

The database is used for:

* Job metadata
* Job status
* Execution result
* Exit code
* Timestamps
* Job locking
* Worker-related state

A simplified Job lifecycle looks like:

```text
             PostgreSQL
                  │
       ┌──────────┴──────────┐
       │                     │
       ▼                     ▼
    Create                 Update
       │                     │
       ▼                     ▼
    Queued ──────────────► Running
                              │
                    ┌─────────┼─────────┐
                    ▼         ▼         ▼
                Completed   Failed   Timeout
```

### Atomic Job Acquisition

Workers use conditional database updates when acquiring jobs.

Conceptually:

```sql
UPDATE jobs
SET
    status = 'running',
    locked_at = NOW(),
    lock_token = $2,
    updated_at = NOW()
WHERE id = $1
  AND (
      status = 'queued'
      OR (
          status = 'running'
          AND locked_at < NOW() - INTERVAL '30 seconds'
      )
  );
```

This allows the database to act as the concurrency control mechanism and prevents multiple Workers from normally claiming the same Job.

## 🔒 Reliability

The Server is designed around the assumption that Workers can fail.

A Worker may disappear while processing a Job:

```text
Job
 │
 ▼
Running
 │
 │ Worker crashes
 ▼
Running
 │
 │ lock expires
 ▼
Recoverable
 │
 ▼
Another Worker
 │
 ▼
Running
```

The persistent Job state allows the execution system to recover from Worker failures without losing the Job record.

## 📊 Observability

The Server exposes Prometheus-compatible metrics.

Example:

```text
GET /metrics
```

Metrics can be collected by Prometheus and visualized using Grafana.

```text
┌──────────────┐
│    Server    │
└──────┬───────┘
       │
       │ /metrics
       ▼
┌──────────────┐
│  Prometheus  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Grafana    │
└──────────────┘
```

The monitoring system can be used to observe:

* Job creation
* Job completion
* Job failures
* Running jobs
* Job execution duration
* Server activity

## 🛠️ Tech Stack

| Component       | Technology     |
| --------------- | -------------- |
| Language        | Rust           |
| HTTP Framework  | Axum           |
| Async Runtime   | Tokio          |
| Message Queue   | Kafka          |
| Database        | PostgreSQL     |
| Database Access | SQLx           |
| Serialization   | Serde          |
| Metrics         | Prometheus     |
| Logging         | tracing        |
| Infrastructure  | Docker Compose |

## 🚀 Getting Started

### Requirements

* Rust
* Docker
* Docker Compose
* PostgreSQL
* Kafka

### Start Infrastructure

```bash
docker compose up -d
```

This starts the development infrastructure required by the Server.

### Configure Environment

Example:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/code_execution
KAFKA_BROKERS=localhost:9092
```

Adjust the values according to your local environment.

### Run the Server

```bash
cargo run
```

The server starts the HTTP API and exposes its endpoints for job submission and querying.

## 🧪 Development

Run the test suite:

```bash
cargo test
```

Check formatting:

```bash
cargo fmt --check
```

Run Clippy:

```bash
cargo clippy
```

## 📁 Project Structure

The exact structure may evolve, but the server is organized around several core responsibilities:

```text
src/
├── api/
│   └── HTTP handlers and routes
│
├── kafka/
│   └── Kafka producer
│
├── jobs/
│   └── Job domain logic
│
├── store/
│   └── PostgreSQL persistence
│
├── metrics/
│   └── Prometheus metrics
│
├── state/
│   └── Application state
│
└── main.rs
```

The goal is to keep HTTP handling, persistence, messaging, and domain logic separated rather than placing the entire application in the Axum handlers.

## 🎯 Design Goals

The Server is intentionally separated from the execution Workers.

This provides several advantages:

### Independent scaling

```text
                  Server
                     │
                  Kafka
                     │
       ┌─────────────┼─────────────┐
       ▼             ▼             ▼
   Worker 1      Worker 2      Worker N
```

The number of Workers can be increased without changing the API layer.

### Failure isolation

A Worker crash should not bring down the HTTP Server.

Likewise, restarting the Server does not require all Workers to restart.

### Asynchronous execution

The Server only handles job submission and state management. Long-running compilation and execution happen asynchronously in Workers.

## 🗺️ Roadmap

* [x] Axum HTTP API
* [x] Job creation
* [x] Job querying
* [x] PostgreSQL persistence
* [x] Kafka producer
* [x] Asynchronous Worker execution
* [x] Job state management
* [x] Job locking
* [x] Job cancellation
* [x] Prometheus metrics
* [x] Docker Compose development environment
* [ ] Authentication / authorization
* [ ] API rate limiting
* [ ] API integration tests
* [ ] End-to-end tests
* [ ] OpenAPI documentation

## 📚 Related Projects

This repository contains the Server component of the code execution platform.

The Worker is responsible for consuming jobs from Kafka and executing user code inside isolated Docker containers.

```text
Code Execution Platform
│
├── server
│   └── HTTP API / Job Management / Kafka Producer
│
└── worker
    └── Kafka Consumer / Execution / Docker Sandbox
```
