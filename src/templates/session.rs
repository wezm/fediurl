use rocket::form::Context;

use crate::form::ContextExt;
use crate::templates::form::field_errors;

markup::define! {
    New<'a, 'v>(context: &'a Context<'v>) {
        form."form-narrow"[action = uri!(crate::web::session::create).to_string(), method="post"] {
            label[for="instance"] { "Instance" }
            input."text-field-short"[type="text", id="instance", name="instance", value=context.value_for("instance"), tabindex=1];
            @field_errors(&context, "instance")
            p."field-description" { "Such as 'mastodon.social'." }

            div.buttons {
                input[type="submit", name="submit", value="Log in", tabindex=3];
            }
        }
    }

    Logout {
        form."form-logout"[action = uri!(crate::web::session::delete).to_string(), method="post"] {
            input[type="hidden", name="_method", value="delete"];
            input[type="submit", name="submit", value="Log out"];
        }
    }
}
