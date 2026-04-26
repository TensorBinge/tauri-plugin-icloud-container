const COMMANDS: &[&str] = &[
    "get_container_status",
    "get_container_url",
    "read_file",
    "write_file",
    "create_file",
    "item_exists",
    "get_attributes",
    "create_directory",
    "list_directory",
    "delete_item",
    "trash_item",
    "move_item",
    "copy_item",
    "get_item_sync_status",
    "start_download",
    "evict_item",
    "is_ubiquitous",
    "watch_directory",
    "unwatch",
    "watch_file",
    "unwatch_file",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}
