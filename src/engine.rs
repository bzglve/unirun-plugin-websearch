use unirun_if::package::Hit;

#[derive(Debug, Clone)]
pub struct Engine {
    name: String,
    /// format string
    url: String,
    pub query: String,
    icon: Option<String>,
}

impl Engine {
    pub fn new(name: &str, url: &str, icon: Option<&str>) -> Self {
        Self {
            name: name.to_owned(),
            url: url.to_owned(),
            query: "".to_owned(),
            icon: icon.map(str::to_owned),
        }
    }

    pub fn with_query(self, query: &str) -> Self {
        Self {
            query: query.to_owned(),
            ..self
        }
    }

    pub fn build(&self) -> String {
        format!("https://{}{}", self.url, self.query)
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::new("Google", "google.com/search?q=", None),
            Self::new("Bing", "www.bing.com/search?q=", None),
            // TODO what about something like bangs?
            Self::new("DuckDuckGo", "duckduckgo.com/?q=", None),
        ]
    }
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.name, self.url)
    }
}

impl From<Engine> for Hit {
    fn from(value: Engine) -> Self {
        Self::new(
            &value.query,
            Some(&format!("Search with {}", value.name)),
            value.icon.as_deref(),
            false,
        )
    }
}

impl From<&Engine> for Hit {
    fn from(value: &Engine) -> Self {
        Self::new(
            &value.query,
            Some(&format!("Search with {}", value.name)),
            value.icon.as_deref(),
            false,
        )
    }
}
