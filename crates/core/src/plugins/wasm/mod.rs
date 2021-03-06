/// The plugin system schema version that is incremented
/// when there are any breaking changes.
pub const PLUGIN_SYSTEM_SCHEMA_VERSION: u32 = 3;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod macros {
    #[macro_export]
    macro_rules! generate_plugin_code {
        () => {
            // HOST FORMATTING

            fn format_with_host(file_path: &PathBuf, file_text: String, override_config: &dprint_core::configuration::ConfigKeyMap) -> Result<String, String> {
                #[link(wasm_import_module = "dprint")]
                extern "C" {
                    fn host_clear_bytes(length: u32);
                    fn host_read_buffer(
                        pointer: u32,
                        length: u32,
                    );
                    fn host_write_buffer(
                        pointer: u32,
                        offset: u32,
                        length: u32,
                    );
                    fn host_take_file_path();
                    fn host_take_override_config();
                    fn host_format() -> u8;
                    fn host_get_formatted_text() -> u32;
                    fn host_get_error_text() -> u32;
                }

                if !override_config.is_empty() {
                    send_string_to_host(serde_json::to_string(override_config).unwrap());
                    unsafe { host_take_override_config(); }
                }

                send_string_to_host(file_path.to_string_lossy().to_string());
                unsafe { host_take_file_path(); }
                send_string_to_host(file_text.clone());

                return match unsafe { host_format() } {
                    0 => { // no change
                        Ok(file_text)
                    },
                    1 => { // change
                        let length = unsafe { host_get_formatted_text() };
                        let formatted_text = get_string_from_host(length);
                        Ok(formatted_text)
                    },
                    2 => { // error
                        let length = unsafe { host_get_error_text() };
                        let error_text = get_string_from_host(length);
                        Err(error_text)
                    },
                    _ => unreachable!(),
                };

                fn send_string_to_host(text: String) {
                    let mut index = 0;
                    let length = set_shared_bytes_str(text);
                    unsafe { host_clear_bytes(length as u32); }
                    while index < length {
                        let read_count = std::cmp::min(length - index, WASM_MEMORY_BUFFER_SIZE);
                        set_buffer_with_shared_bytes(index, read_count);
                        unsafe { host_read_buffer(get_wasm_memory_buffer() as u32, read_count as u32); }
                        index += read_count;
                    }
                }

                fn get_string_from_host(length: u32) -> String {
                    let mut index: u32 = 0;
                    clear_shared_bytes(length as usize);
                    while index < length {
                        let read_count = std::cmp::min(length - index, WASM_MEMORY_BUFFER_SIZE as u32);
                        unsafe { host_write_buffer(get_wasm_memory_buffer() as u32, index, read_count); }
                        add_to_shared_bytes_from_buffer(read_count as usize);
                        index += read_count;
                    }
                    take_string_from_shared_bytes()
                }
            }

            // FORMATTING

            static mut OVERRIDE_CONFIG: Option<dprint_core::configuration::ConfigKeyMap> = None;
            static mut FILE_PATH: Option<PathBuf> = None;
            static mut FORMATTED_TEXT: Option<String> = None;
            static mut ERROR_TEXT: Option<String> = None;

            #[no_mangle]
            pub fn set_override_config() {
                let bytes = take_from_shared_bytes();
                let config = serde_json::from_slice(&bytes).unwrap();
                unsafe { OVERRIDE_CONFIG.replace(config) };
            }

            #[no_mangle]
            pub fn set_file_path() {
                let text = take_string_from_shared_bytes();
                unsafe { FILE_PATH.replace(PathBuf::from(text)) };
            }

            #[no_mangle]
            pub fn format() -> u8 {
                ensure_initialized();
                let config = unsafe {
                    if let Some(override_config) = OVERRIDE_CONFIG.take() {
                        std::borrow::Cow::Owned(create_resolved_config_result(override_config).config)
                    } else {
                        std::borrow::Cow::Borrowed(&get_resolved_config_result().config)
                    }
                };
                let file_path = unsafe { FILE_PATH.take().expect("Expected the file path to be set.") };
                let file_text = take_string_from_shared_bytes();

                let formatted_text = format_text(&file_path, &file_text, &config);
                match formatted_text {
                    Ok(formatted_text) => {
                        if formatted_text == file_text {
                            0 // no change
                        } else {
                            unsafe { FORMATTED_TEXT.replace(formatted_text) };
                            1 // change
                        }
                    },
                    Err(err_text) => {
                        unsafe { ERROR_TEXT.replace(err_text) };
                        2 // error
                    }
                }
            }

            #[no_mangle]
            pub fn get_formatted_text() -> usize {
                let formatted_text = unsafe { FORMATTED_TEXT.take().expect("Expected to have formatted text.") };
                set_shared_bytes_str(formatted_text)
            }

            #[no_mangle]
            pub fn get_error_text() -> usize {
                let error_text = unsafe { ERROR_TEXT.take().expect("Expected to have error text.") };
                set_shared_bytes_str(error_text)
            }

            // INFORMATION & CONFIGURATION

            static mut RESOLVE_CONFIGURATION_RESULT: Option<dprint_core::configuration::ResolveConfigurationResult<Configuration>> = None;

            #[no_mangle]
            pub fn get_plugin_info() -> usize {
                use dprint_core::plugins::PluginInfo;
                let info_json = serde_json::to_string(&PluginInfo {
                    name: String::from(env!("CARGO_PKG_NAME")),
                    version: String::from(env!("CARGO_PKG_VERSION")),
                    config_key: get_plugin_config_key(),
                    file_extensions: get_plugin_file_extensions(),
                    help_url: get_plugin_help_url(),
                    config_schema_url: get_plugin_config_schema_url(),
                }).unwrap();
                set_shared_bytes_str(info_json)
            }

            #[no_mangle]
            pub fn get_license_text() -> usize {
                set_shared_bytes_str(get_plugin_license_text())
            }

            #[no_mangle]
            pub fn get_resolved_config() -> usize {
                let bytes = serde_json::to_vec(&get_resolved_config_result().config).unwrap();
                set_shared_bytes(bytes)
            }

            #[no_mangle]
            pub fn get_config_diagnostics() -> usize {
                let bytes = serde_json::to_vec(&get_resolved_config_result().diagnostics).unwrap();
                set_shared_bytes(bytes)
            }

            fn get_resolved_config_result<'a>() -> &'a dprint_core::configuration::ResolveConfigurationResult<Configuration> {
                unsafe {
                    ensure_initialized();
                    return RESOLVE_CONFIGURATION_RESULT.as_ref().unwrap();
                }
            }

            fn ensure_initialized() {
                unsafe {
                    if RESOLVE_CONFIGURATION_RESULT.is_none() {
                        let config_result = create_resolved_config_result(std::collections::HashMap::new());
                        RESOLVE_CONFIGURATION_RESULT.replace(config_result);
                    }
                }
            }

            fn create_resolved_config_result(override_config: dprint_core::configuration::ConfigKeyMap) -> dprint_core::configuration::ResolveConfigurationResult<Configuration> {
                unsafe {
                    if let Some(global_config) = &GLOBAL_CONFIG {
                        if let Some(plugin_config) = &PLUGIN_CONFIG {
                            let mut plugin_config = plugin_config.clone();
                            for (key, value) in override_config {
                                plugin_config.insert(key, value);
                            }
                            return resolve_config(plugin_config, global_config);
                        }
                    }
                }

                panic!("Plugin must have global config and plugin config set before use.");
            }

            // INITIALIZATION

            static mut GLOBAL_CONFIG: Option<dprint_core::configuration::GlobalConfiguration> = None;
            static mut PLUGIN_CONFIG: Option<dprint_core::configuration::ConfigKeyMap> = None;

            #[no_mangle]
            pub fn set_global_config() {
                let bytes = take_from_shared_bytes();
                let global_config: dprint_core::configuration::GlobalConfiguration = serde_json::from_slice(&bytes).unwrap();
                unsafe {
                    GLOBAL_CONFIG.replace(global_config);
                    RESOLVE_CONFIGURATION_RESULT.take(); // clear
                }
            }

            #[no_mangle]
            pub fn set_plugin_config() {
                let bytes = take_from_shared_bytes();
                let plugin_config: dprint_core::configuration::ConfigKeyMap = serde_json::from_slice(&bytes).unwrap();
                unsafe {
                    PLUGIN_CONFIG.replace(plugin_config);
                    RESOLVE_CONFIGURATION_RESULT.take(); // clear
                }
            }

            // LOW LEVEL SENDING AND RECEIVING

            const WASM_MEMORY_BUFFER_SIZE: usize = 4 * 1024;
            static mut WASM_MEMORY_BUFFER: [u8; WASM_MEMORY_BUFFER_SIZE] = [0; WASM_MEMORY_BUFFER_SIZE];
            static mut SHARED_BYTES: Vec<u8> = Vec::new();

            #[no_mangle]
            pub fn get_plugin_schema_version() -> u32 {
                dprint_core::plugins::wasm::PLUGIN_SYSTEM_SCHEMA_VERSION // version 1
            }

            #[no_mangle]
            pub fn get_wasm_memory_buffer() -> *const u8 {
                unsafe { WASM_MEMORY_BUFFER.as_ptr() }
            }

            #[no_mangle]
            pub fn get_wasm_memory_buffer_size() -> usize {
                WASM_MEMORY_BUFFER_SIZE
            }

            #[no_mangle]
            pub fn add_to_shared_bytes_from_buffer(length: usize) {
                unsafe {
                    SHARED_BYTES.extend(&WASM_MEMORY_BUFFER[..length])
                }
            }

            #[no_mangle]
            pub fn set_buffer_with_shared_bytes(offset: usize, length: usize) {
                unsafe {
                    let bytes = &SHARED_BYTES[offset..(offset+length)];
                    &WASM_MEMORY_BUFFER[..length].copy_from_slice(bytes);
                }
            }

            #[no_mangle]
            pub fn clear_shared_bytes(capacity: usize) {
                unsafe { SHARED_BYTES = Vec::with_capacity(capacity); }
            }

            fn take_string_from_shared_bytes() -> String {
                String::from_utf8(take_from_shared_bytes()).unwrap()
            }

            fn take_from_shared_bytes() -> Vec<u8> {
                unsafe {
                    std::mem::replace(&mut SHARED_BYTES, Vec::with_capacity(0))
                }
            }

            fn set_shared_bytes_str(text: String) -> usize {
                set_shared_bytes(text.into_bytes())
            }

            fn set_shared_bytes(bytes: Vec<u8>) -> usize {
                let length = bytes.len();
                unsafe { SHARED_BYTES = bytes }
                length
            }
        }
   }
}
