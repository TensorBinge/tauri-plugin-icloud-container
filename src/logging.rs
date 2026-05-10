pub fn format_log_line(level: &str, scope: &str, event: &str, fields: &[(&str, String)]) -> String {
    let suffix = fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ");

    if suffix.is_empty() {
        format!("[{level}] [{scope}] {event}")
    } else {
        format!("[{level}] [{scope}] {event} {suffix}")
    }
}

pub(crate) fn serialize_log_value<T>(value: &T) -> String
where
    T: serde::Serialize + ?Sized,
{
    serde_json::to_string(value).unwrap_or_else(|_| r#"\"<unserializable>\""#.to_string())
}

pub(crate) fn info(scope: &str, event: &str, fields: &[(&str, String)]) {
    log::info!("{}", format_log_line("info", scope, event, fields));
}

#[allow(dead_code)]
pub(crate) fn warn(scope: &str, event: &str, fields: &[(&str, String)]) {
    log::warn!("{}", format_log_line("warn", scope, event, fields));
}

#[macro_export]
macro_rules! plugin_log_info {
    ($scope:expr, $event:expr $(, $key:expr => $value:expr )* $(,)?) => {{
        let fields: Vec<(&str, String)> = vec![$(($key, $crate::logging::serialize_log_value(&$value))),*];
        $crate::logging::info($scope, $event, &fields);
    }};
}

#[macro_export]
macro_rules! plugin_log_warn {
    ($scope:expr, $event:expr $(, $key:expr => $value:expr )* $(,)?) => {{
        let fields: Vec<(&str, String)> = vec![$(($key, $crate::logging::serialize_log_value(&$value))),*];
        $crate::logging::warn($scope, $event, &fields);
    }};
}
