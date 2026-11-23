use jiff::Timestamp;

pub(crate) fn now() -> String {
    format!("{:.3}", Timestamp::now())
}
