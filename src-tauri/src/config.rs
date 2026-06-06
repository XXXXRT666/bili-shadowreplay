use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{self, AtomicU64};
use std::sync::Arc;

use crate::{danmu2ass::Danmu2AssOptions, recorder_manager::ClipRangeParams};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub cache: String,
    pub output: String,
    pub live_start_notify: bool,
    pub live_end_notify: bool,
    pub clip_notify: bool,
    pub post_notify: bool,
    #[serde(default = "default_auto_subtitle")]
    pub auto_subtitle: bool,
    #[serde(default = "default_subtitle_generator_type")]
    pub subtitle_generator_type: String,
    #[serde(default = "default_whisper_model")]
    pub whisper_model: String,
    #[serde(default = "default_whisper_prompt")]
    pub whisper_prompt: String,
    #[serde(default = "default_openai_api_endpoint")]
    pub openai_api_endpoint: String,
    #[serde(default = "default_openai_api_key")]
    pub openai_api_key: String,
    #[serde(default = "default_online_asr_model")]
    pub online_asr_model: String,
    #[serde(default = "default_asr_hotwords")]
    pub asr_hotwords: AsrHotwordConfig,
    #[serde(default = "default_oss_access_key_id")]
    pub oss_access_key_id: String,
    #[serde(default = "default_oss_access_key_secret")]
    pub oss_access_key_secret: String,
    #[serde(default = "default_oss_bucket")]
    pub oss_bucket: String,
    #[serde(default = "default_oss_endpoint")]
    pub oss_endpoint: String,
    #[serde(default = "default_oss_object_prefix")]
    pub oss_object_prefix: String,
    #[serde(default = "default_clip_name_format")]
    pub clip_name_format: String,
    #[serde(default = "default_auto_generate_config")]
    pub auto_generate: AutoGenerateConfig,
    #[serde(default = "default_status_check_interval")]
    pub status_check_interval: u64,
    #[serde(skip)]
    pub config_path: String,
    #[serde(default = "default_whisper_language")]
    pub whisper_language: String,
    #[serde(default = "default_webhook_url")]
    pub webhook_url: String,
    #[serde(default = "default_danmu_ass_options")]
    pub danmu_ass_options: Danmu2AssOptions,
    #[serde(skip)]
    pub update_interval: Arc<AtomicU64>,
    #[serde(default = "default_powerlive_key")]
    pub powerlive_key: String,
    #[serde(default = "default_use_native_clip_player")]
    pub use_native_clip_player: bool,
    #[serde(default = "default_native_clip_player_windowed_offset")]
    pub native_clip_player_windowed_offset: i32,
    #[serde(default = "default_use_seekbar_thumbnail_cache")]
    pub use_seekbar_thumbnail_cache: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct AutoGenerateConfig {
    pub enabled: bool,
    pub encode_danmu: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AsrHotwordConfig {
    #[serde(default = "default_asr_hotword_prefix")]
    pub prefix: String,
    #[serde(default)]
    pub vocabulary_id: String,
    #[serde(default)]
    pub vocabulary_signature: String,
    #[serde(default)]
    pub target_model: String,
    #[serde(default)]
    pub words: Vec<AsrHotword>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AsrHotword {
    pub text: String,
    pub weight: u8,
    pub lang: String,
}

fn default_danmu_ass_options() -> Danmu2AssOptions {
    Danmu2AssOptions::default()
}

fn default_auto_subtitle() -> bool {
    false
}

fn default_subtitle_generator_type() -> String {
    "whisper".to_string()
}

fn default_whisper_model() -> String {
    "whisper_model.bin".to_string()
}

fn default_whisper_prompt() -> String {
    "".to_string()
}

fn default_openai_api_endpoint() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_api_key() -> String {
    String::new()
}

fn default_online_asr_model() -> String {
    "whisper-1".to_string()
}

fn default_asr_hotword_prefix() -> String {
    "bsrasr".to_string()
}

fn default_asr_hotwords() -> AsrHotwordConfig {
    AsrHotwordConfig {
        prefix: default_asr_hotword_prefix(),
        vocabulary_id: String::new(),
        vocabulary_signature: String::new(),
        target_model: String::new(),
        words: Vec::new(),
    }
}

fn default_oss_access_key_id() -> String {
    String::new()
}

fn default_oss_access_key_secret() -> String {
    String::new()
}

fn default_oss_bucket() -> String {
    String::new()
}

fn default_oss_endpoint() -> String {
    "https://oss-cn-beijing.aliyuncs.com".to_string()
}

fn default_oss_object_prefix() -> String {
    "bili-shadowreplay/asr".to_string()
}

fn default_clip_name_format() -> String {
    "[{room_id}][{note}][{live_id}][{title}][{created_at}].mp4".to_string()
}

fn default_auto_generate_config() -> AutoGenerateConfig {
    AutoGenerateConfig {
        enabled: false,
        encode_danmu: false,
    }
}

fn default_status_check_interval() -> u64 {
    30
}

fn default_whisper_language() -> String {
    "auto".to_string()
}

fn default_webhook_url() -> String {
    String::new()
}

fn default_powerlive_key() -> String {
    String::new()
}

fn default_use_native_clip_player() -> bool {
    true
}

fn default_native_clip_player_windowed_offset() -> i32 {
    28
}

fn default_use_seekbar_thumbnail_cache() -> bool {
    true
}

impl Config {
    pub fn load(
        config_path: &PathBuf,
        default_cache: &Path,
        default_output: &Path,
    ) -> Result<Self, String> {
        if let Ok(content) = std::fs::read_to_string(config_path) {
            if let Ok(mut config) = toml::from_str::<Config>(&content) {
                config.config_path = config_path.to_str().unwrap().into();
                config.update_interval = Arc::new(AtomicU64::new(config.status_check_interval));
                return Ok(config);
            }
        }

        if let Some(dir_path) = PathBuf::from(config_path).parent() {
            if let Err(e) = std::fs::create_dir_all(dir_path) {
                return Err(format!("Failed to create config dir: {e}"));
            }
        }

        let config = Config {
            cache: default_cache.to_str().unwrap().into(),
            output: default_output.to_str().unwrap().into(),
            live_start_notify: true,
            live_end_notify: true,
            clip_notify: true,
            post_notify: true,
            auto_subtitle: false,
            subtitle_generator_type: default_subtitle_generator_type(),
            whisper_model: default_whisper_model(),
            whisper_prompt: default_whisper_prompt(),
            openai_api_endpoint: default_openai_api_endpoint(),
            openai_api_key: default_openai_api_key(),
            online_asr_model: default_online_asr_model(),
            asr_hotwords: default_asr_hotwords(),
            oss_access_key_id: default_oss_access_key_id(),
            oss_access_key_secret: default_oss_access_key_secret(),
            oss_bucket: default_oss_bucket(),
            oss_endpoint: default_oss_endpoint(),
            oss_object_prefix: default_oss_object_prefix(),
            clip_name_format: default_clip_name_format(),
            auto_generate: default_auto_generate_config(),
            status_check_interval: default_status_check_interval(),
            config_path: config_path.to_str().unwrap().into(),
            whisper_language: default_whisper_language(),
            webhook_url: default_webhook_url(),
            danmu_ass_options: default_danmu_ass_options(),
            update_interval: Arc::new(AtomicU64::new(default_status_check_interval())),
            powerlive_key: default_powerlive_key(),
            use_native_clip_player: default_use_native_clip_player(),
            native_clip_player_windowed_offset: default_native_clip_player_windowed_offset(),
            use_seekbar_thumbnail_cache: default_use_seekbar_thumbnail_cache(),
        };

        config.save();

        Ok(config)
    }

    pub fn save(&self) {
        let content = toml::to_string(&self).unwrap();
        if let Err(e) = std::fs::write(self.config_path.clone(), content) {
            log::error!("Failed to save config: {} {}", e, self.config_path);
        }
    }

    #[allow(dead_code)]
    pub fn set_cache_path(&mut self, path: &str) {
        self.cache = path.to_string();
        self.save();
    }

    #[allow(dead_code)]
    pub fn set_output_path(&mut self, path: &str) {
        self.output = path.into();
        self.save();
    }

    #[allow(dead_code)]
    pub fn set_whisper_language(&mut self, language: &str) {
        self.whisper_language = language.to_string();
        self.save();
    }

    #[allow(dead_code)]
    pub fn set_danmu_ass_options(&mut self, options: Danmu2AssOptions) {
        self.danmu_ass_options = options;
        self.save();
    }

    pub fn generate_clip_name(&self, params: &ClipRangeParams) -> PathBuf {
        // get format config
        // filter special characters from title to make sure file name is valid
        let title = params
            .title
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();
        let format_config = self.clip_name_format.clone();
        let format_config = format_config.replace("{title}", &title);
        let format_config = format_config.replace("{platform}", &params.platform);
        let format_config = format_config.replace("{room_id}", &params.room_id.to_string());
        let format_config = format_config.replace("{live_id}", &params.live_id);
        let format_config = format_config.replace("{note}", &params.note);
        let format_config = format_config.replace(
            "{x}",
            &params
                .ranges
                .first()
                .map_or("0".to_string(), |r| r.start.to_string()),
        );
        let format_config = format_config.replace(
            "{y}",
            &params
                .ranges
                .last()
                .map_or("0".to_string(), |r| r.end.to_string()),
        );
        let format_config = format_config.replace(
            "{created_at}",
            &Local::now().format("%Y-%m-%d_%H-%M-%S").to_string(),
        );
        let duration = params.ranges.iter().map(|r| r.duration()).sum::<f64>();
        let format_config = format_config.replace("{length}", &duration.to_string());

        let mut format_config = format_config;
        while format_config.contains("[]") {
            format_config = format_config.replace("[]", "");
        }

        let sanitized = sanitize_filename::sanitize(&format_config);
        let output = self.output.clone();
        let grouped_dir = Path::new(&output)
            .join(sanitize_filename::sanitize(&params.platform))
            .join(sanitize_filename::sanitize(&params.room_id));

        grouped_dir.join(&sanitized)
    }

    pub fn set_status_check_interval(&mut self, interval: u64) {
        self.status_check_interval = interval;
        self.update_interval
            .store(interval, atomic::Ordering::Relaxed);
        self.save();
    }
}
