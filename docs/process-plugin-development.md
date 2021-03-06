# Creating a Process Plugin (Schema Version 2)

Process plugins are created (as opposed to the recommended WASM plugins), when the language does not have good support for compiling to a single _.wasm_ file.

## Rust - Using `dprint-core`

Implementing a Process plugin is easy if you're using Rust as there are several helpers in `dprint-core`.

1. Use the `process` feature from `dprint-core` in _Cargo.toml_:

   ```toml
   dprint-core = { version = "...", features = ["process"] }
   serde = { version = "1.0.88", features = ["derive"] }
   serde_json = "1.0"
   ```

2. Create a `Configuration` struct somewhere in your project:

   ```rust
   use serde::{Serialize, Deserialize};

   #[derive(Clone, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct Configuration {
       // add configuration properties here...
       line_width: u32, // for example
   }
   ```

3. Implement `ProcessPluginHandler`

   ```rust
   use std::path::PathBuf;
   use std::collections::HashMap;

   use dprint_core::configuration::{GlobalConfiguration, ResolveConfigurationResult, get_unknown_property_diagnostics, ConfigKeyMap, get_value};
   use dprint_core::err;
   use dprint_core::types::ErrBox;
   use dprint_core::plugins::PluginInfo;
   use dprint_core::plugins::process::ProcessPluginHandler;

   use super::configuration::Configuration; // import the Configuration from above somehow

   pub struct MyProcessPluginHandler {
   }

   impl MyProcessPluginHandler {
       fn new() -> Self {
           MyProcessPluginHandler {}
       }
   }

   impl ProcessPluginHandler<Configuration> for MyProcessPluginHandler {
       fn get_plugin_info(&self) -> PluginInfo {
           PluginInfo {
               name: String::from(env!("CARGO_PKG_NAME")),
               version: String::from(env!("CARGO_PKG_VERSION")),
               config_key: "keyGoesHere".to_string(),
               file_extensions: vec!["txt_ps".to_string()],
               help_url: "".to_string(), // fill this in
               config_schema_url: "".to_string()
           }
       }

       fn get_license_text(&self) -> &str {
           "License text goes here."
       }

       fn resolve_config(&self, config: ConfigKeyMap, global_config: &GlobalConfiguration) -> ResolveConfigurationResult<Configuration> {
           // implement this... for example
           let mut config = config;
           let mut diagnostics = Vec::new();
           let line_width = get_value(&mut config, "line_width", global_config.line_width.unwrap_or(120), &mut diagnostics);

           diagnostics.extend(get_unknown_property_diagnostics(config));

           ResolveConfigurationResult {
               config: Configuration { ending, line_width },
               diagnostics,
           }
       }

       fn format_text<'a>(
           &'a self,
           file_path: &PathBuf,
           file_text: &str,
           config: &Configuration,
           format_with_host: Box<dyn FnMut(&PathBuf, String, &ConfigKeyMap) -> Result<String, ErrBox> + 'a>,
       ) -> Result<String, ErrBox> {
           // format here
       }
   }
   ```

4. In your plugin's `main` function, parse out the `--parent-pid` argument and using that argument, start a thread that periodically checks for the existence of that process. When the process no longer exists, then it should exit the current process. This helps prevent a process from running without ever closing. Implementing this is easy with `dprint-core` as you just need to run the `start_parent_process_checker_thread` function:

   ```rust
   use dprint_core::plugins::process::start_parent_process_checker_thread;

   let parent_process_id = ...; // parse this from the `--parent-pid` command line argument
   start_parent_process_checker_thread(String::from(env!("CARGO_PKG_NAME")), parent_process_id);
   ```

5. Finally, use your created plugin handler to start reading and writing to stdin and stdout:

   ```rust
   handle_process_stdin_stdout_messages(MyProcessPluginHandler::new())
   ```

## Schema Version 2 Overview

TODO...

### Creating a `.exe-plugin` file

TODO...
