use rocket::form::Context;

markup::define! {
    FieldErrors<'a, 'v>(context: &'a Context<'v>, name: &'a str) {
        @for error in context.field_errors(name) {
            span."validation-error" { @error.to_string() }
        }
    }
}

pub fn field_errors<'a, 'v>(context: &'a Context<'v>, name: &'a str) -> FieldErrors<'a, 'v> {
    FieldErrors { context, name }
}
