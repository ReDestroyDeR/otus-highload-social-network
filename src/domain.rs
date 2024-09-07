pub(crate) mod protocol {
    use warp::{reject, Rejection, Reply};

    pub trait ToReply {
        fn into_reply(self) -> impl Reply + 'static;
    }

    pub trait ToResponse {
        fn into_response(self) -> Result<Box<dyn Reply>, Rejection>;
    }

    impl<T: ToReply> ToResponse for Option<T> {
        fn into_response(self) -> Result<Box<dyn Reply>, Rejection> {
            match self {
                None => Err(reject::not_found()),
                Some(underlying) => Ok(Box::new(underlying.into_reply())),
            }
        }
    }

    impl<T, E> ToResponse for Result<T, E>
    where
        T: ToReply,
        E: ToReply,
    {
        fn into_response(self) -> Result<Box<dyn Reply>, Rejection> {
            match self {
                Err(error) => Ok(Box::new(error.into_reply())),
                Ok(underlying) => Ok(Box::new(underlying.into_reply())),
            }
        }
    }
}

pub(crate) mod user {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgTypeInfo;
    use sqlx::{Decode, Encode, FromRow, Postgres, Type};
    use uuid::Uuid;
    use warp::{reply, Reply};

    use crate::domain::protocol::ToReply;
    use crate::domain::user::Gender::Unknown;

    #[derive(Serialize, FromRow, Clone)]
    pub struct User {
        pub id: Uuid,
        pub first_name: String,
        pub last_name: String,
        pub birth_date: NaiveDate,
        pub gender: Gender,
        pub interests: Vec<Interest>,
        pub city: String,
    }

    impl ToReply for User {
        fn into_reply(self) -> impl Reply {
            reply::json(&self)
        }
    }

    #[derive(Serialize, Deserialize, Decode, Encode, Clone)]
    pub enum Gender {
        Male,
        Female,
        Unknown,
    }

    impl From<String> for Gender {
        fn from(value: String) -> Self {
            match value.as_str() {
                "Male" => Gender::Male,
                "Female" => Gender::Female,
                _ => Unknown,
            }
        }
    }

    impl Into<String> for &Gender {
        fn into(self) -> String {
            match self {
                Gender::Male => "Male".to_owned(),
                Gender::Female => "Female".to_owned(),
                _ => "Unknown".to_owned(),
            }
        }
    }

    impl Type<Postgres> for Gender {
        fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
            PgTypeInfo::with_name("VARCHAR")
        }
    }

    #[derive(Serialize, Deserialize, Type, FromRow, Clone)]
    pub struct Interest {
        pub name: String,
        pub description: String,
    }

    #[derive(Serialize)]
    pub struct AuthenticationResponse {
        pub(crate) session_id: String,
    }

    impl ToReply for AuthenticationResponse {
        fn into_reply(self) -> impl Reply {
            reply::json(&self)
        }
    }

    #[derive(Deserialize)]
    pub struct RegistrationRequest {
        pub credentials: Credentials,
        pub first_name: String,
        pub last_name: String,
        pub birth_date: NaiveDate,
        pub gender: Gender,
        pub interests: Vec<Interest>,
        pub city: String,
    }

    #[derive(Deserialize)]
    pub struct Credentials {
        pub login: String,
        pub password: String,
    }
}
