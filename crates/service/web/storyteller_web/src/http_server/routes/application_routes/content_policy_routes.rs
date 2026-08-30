use actix_http::body::MessageBody;
use actix_service::ServiceFactory;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{web, App, Error, HttpResponse};

use crate::http_server::endpoints::content_policy::record_content_policy_violation_handler::record_content_policy_violation_handler;

pub fn add_content_policy_routes<T, B>(app: App<T>) -> App<T>
where
  B: MessageBody,
  T: ServiceFactory<
    ServiceRequest,
    Config = (),
    Response = ServiceResponse<B>,
    Error = Error,
    InitError = (),
  >,
{
  app.service(
    web::scope("/v1/content_policy")
      .service(
        web::resource("/record_violation")
          .route(web::post().to(record_content_policy_violation_handler))
          .route(web::head().to(|| HttpResponse::Ok())),
      ),
  )
}
