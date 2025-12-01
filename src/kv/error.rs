use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("redb error: {source}"))]
    Redb { source: redb::Error },

    #[snafu(display("serialization error: {source}"))]
    Serialization { source: serde_json::Error },

    #[snafu(display("io error: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("raft error: {message}"))]
    Raft { message: String },
}

impl From<redb::Error> for Error {
    fn from(source: redb::Error) -> Self {
        Self::Redb { source }
    }
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io { source }
    }
}
