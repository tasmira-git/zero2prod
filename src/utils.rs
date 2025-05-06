use actix_web::HttpResponse;



pub fn e500<T>(e: T) -> actix_web::Error
where 
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((actix_web::http::header::LOCATION, location))
        .finish()
}