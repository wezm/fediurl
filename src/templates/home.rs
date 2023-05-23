use crate::models::instance::Instance;

markup::define! {
    Home(instance: Option<Instance>) {
        section."text-center"[id="connect"] {
            h2 { "Redirect Mastodon links to your own instance" }

            p.lede { "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat." }

            @match instance {
                Some(instance) => {
                    "You are connected to " @instance.domain
                }
                None => {
                    a."link-button"[href=uri!(crate::web::session::new).to_string()] { span { "Connect Your Instance" } }
                }
            }
        }

        section."integrations"[id="integrations"] {

            h3 { "Integrations" }

            h4 { "Browser Extensions" }

            // <div class="container">
            //     <h2><a href="#browsers">Get 1Password in your browser</a></h2>
            //     <p><strong>1Password works everywhere you do.</strong> Easily sign in to sites, generate strong passwords, and find or autofill what you need in an instant. It’s all at your fingertips.</p>
            // </div>
            div.container {
                ul."device-list" {
                    li.browser {
                        img[src="/img/downloads/chrome-x@3x.007614c27c9d9155ddc87f0c44f874c3.png", alt="Download 1Password extension for Chrome"];
                        a[href="https://chrome.google.com/webstore/detail/1password-–-password-mana/aeblfdkhhhdcdjpifhhbdiojplfjncoa"] { "Install" }
                    }
                    li.browser {
                        img[src="/img/downloads/firefox-x@3x.1f0bf050bd8fba311aff9e189aaf181c.png", alt="Download 1Password extension for Firefox"];
                        a[href="https://addons.mozilla.org/en-US/firefox/addon/1password-x-password-manager/?src=search"] { "Install" }
                    }
                }
            }

            h4 { "Espanso" }

            p { "Espanso is a free and open-source text expansion tool for Linux, macOS, and Windows." }
        }

        section.usage {
            h3 { "How it Works" }

            p {
                "Once Fediurl is connected to your instance you can put "
                a[href="#"] { code { "https://fediurl.com/" } }
                "in front of any Mastodon URL and it will try to find that URL on your instance "
                "and redirect you to it. For example, for the status at https://mastodon.decentralised.social/@wezm/110375901972328927 "
                "the redirect would be "
                code { "https://fediurl.com/https://mastodon.decentralised.social/@wezm/110375901972328927" } "."
                "Use one of the " a[href="#integrations"] { "integrations" } " to automate this."
            }
        }

        section.privacy {
            h3 { "Privacy & Security" }

            p {
                "Privacy and security is any important part of the Fediurl implementation. "
                "The following measures are taken:"
            }

            ul {
                li { "Fediurl requests the bare minimum read-only permissions to perform its function "
                "it can't read or post to your timeline." }
                li { "There is no tracking or analytics used on the site." }
                li { "User tokens are stored encrypted in the database." }
                li { "The code is open-source." }
            }
        }
    }
}
