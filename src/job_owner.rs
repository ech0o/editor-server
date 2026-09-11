use uuid::Uuid;
#[derive(Debug, Clone, Copy)]
pub enum JobOwner {
    User{
        user_id: Uuid,
    },
    Apikey{
        api_key_id: Uuid,
    },
}

impl JobOwner{
    pub fn apply(self, job: &mut crate::jobs::NewJob){
        match self{
            JobOwner::User{user_id}=>{
                job.user_id = Some(user_id);
                job.api_key_id = None;
            },
            JobOwner::Apikey{api_key_id}=>{
                job.user_id = None;
                job.api_key_id = Some(api_key_id);
            }
        }
    }
}