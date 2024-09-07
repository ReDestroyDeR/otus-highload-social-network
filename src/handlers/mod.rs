use warp::Filter;

pub(crate) mod user_handler;

pub trait RestHandler
where
    Self: Send,
{
    fn routes(self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone;
}
