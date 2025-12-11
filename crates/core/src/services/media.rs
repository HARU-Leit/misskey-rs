//! Media processing service for image and video handling.

use serde::{Deserialize, Serialize};
use std::path::Path;

use misskey_common::{AppError, AppResult};

/// Supported image formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// JPEG format
    Jpeg,
    /// PNG format
    Png,
    /// WebP format
    WebP,
    /// AVIF format
    Avif,
    /// GIF format
    Gif,
}

impl ImageFormat {
    /// Get MIME type for this format.
    #[must_use]
    pub const fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::WebP => "image/webp",
            Self::Avif => "image/avif",
            Self::Gif => "image/gif",
        }
    }

    /// Get file extension for this format.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Avif => "avif",
            Self::Gif => "gif",
        }
    }

    /// Detect format from file extension.
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "png" => Some(Self::Png),
            "webp" => Some(Self::WebP),
            "avif" => Some(Self::Avif),
            "gif" => Some(Self::Gif),
            _ => None,
        }
    }

    /// Detect format from MIME type.
    #[must_use]
    pub fn from_mime_type(mime: &str) -> Option<Self> {
        match mime {
            "image/jpeg" => Some(Self::Jpeg),
            "image/png" => Some(Self::Png),
            "image/webp" => Some(Self::WebP),
            "image/avif" => Some(Self::Avif),
            "image/gif" => Some(Self::Gif),
            _ => None,
        }
    }
}

/// Image dimensions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageDimensions {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

/// Thumbnail size presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailSize {
    /// Small thumbnail (150x150)
    Small,
    /// Medium thumbnail (400x400)
    Medium,
    /// Large thumbnail (800x800)
    Large,
    /// Custom size
    Custom(u32, u32),
}

impl ThumbnailSize {
    /// Get dimensions for this size.
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Small => (150, 150),
            Self::Medium => (400, 400),
            Self::Large => (800, 800),
            Self::Custom(w, h) => (*w, *h),
        }
    }
}

/// Image processing options.
#[derive(Debug, Clone)]
pub struct ImageProcessingOptions {
    /// Output format
    pub format: Option<ImageFormat>,
    /// Quality (1-100)
    pub quality: Option<u8>,
    /// Maximum width
    pub max_width: Option<u32>,
    /// Maximum height
    pub max_height: Option<u32>,
    /// Strip metadata
    pub strip_metadata: bool,
    /// Auto-orient based on EXIF
    pub auto_orient: bool,
}

impl Default for ImageProcessingOptions {
    fn default() -> Self {
        Self {
            format: None,
            quality: Some(85),
            max_width: None,
            max_height: None,
            strip_metadata: true,
            auto_orient: true,
        }
    }
}

/// Image metadata extracted from file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    /// Image dimensions
    pub dimensions: ImageDimensions,
    /// Original format
    pub format: ImageFormat,
    /// File size in bytes
    pub file_size: u64,
    /// Has alpha channel
    pub has_alpha: bool,
    /// Is animated (GIF/APNG/WebP)
    pub is_animated: bool,
    /// Blurhash
    pub blurhash: Option<String>,
    /// Dominant color (hex)
    pub dominant_color: Option<String>,
    /// EXIF data (if available and not stripped)
    pub exif: Option<ExifData>,
}

/// EXIF data extracted from images.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExifData {
    /// Camera make
    pub make: Option<String>,
    /// Camera model
    pub model: Option<String>,
    /// Date taken
    pub date_taken: Option<String>,
    /// GPS latitude
    pub latitude: Option<f64>,
    /// GPS longitude
    pub longitude: Option<f64>,
    /// Exposure time
    pub exposure_time: Option<String>,
    /// F-number
    pub f_number: Option<f64>,
    /// ISO speed
    pub iso: Option<u32>,
    /// Focal length
    pub focal_length: Option<f64>,
}

/// Video metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoMetadata {
    /// Duration in seconds
    pub duration: f64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Frame rate
    pub frame_rate: Option<f64>,
    /// Video codec
    pub video_codec: Option<String>,
    /// Audio codec
    pub audio_codec: Option<String>,
    /// Bitrate in kbps
    pub bitrate: Option<u32>,
    /// File size in bytes
    pub file_size: u64,
}

/// Processed image result.
#[derive(Debug)]
pub struct ProcessedImage {
    /// Image data
    pub data: Vec<u8>,
    /// Format
    pub format: ImageFormat,
    /// Dimensions
    pub dimensions: ImageDimensions,
    /// File size
    pub file_size: u64,
}

/// Media processing configuration.
#[derive(Debug, Clone)]
pub struct MediaConfig {
    /// Maximum image dimension (width or height)
    pub max_image_dimension: u32,
    /// Maximum video duration in seconds
    pub max_video_duration: u32,
    /// Enable WebP conversion
    pub enable_webp_conversion: bool,
    /// Enable AVIF conversion
    pub enable_avif_conversion: bool,
    /// Enable video transcoding
    pub enable_video_transcoding: bool,
    /// Thumbnail quality
    pub thumbnail_quality: u8,
    /// Strip image metadata by default
    pub strip_metadata: bool,
    /// `FFmpeg` path
    pub ffmpeg_path: Option<String>,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            max_image_dimension: 4096,
            max_video_duration: 300,
            enable_webp_conversion: true,
            enable_avif_conversion: false,
            enable_video_transcoding: false,
            thumbnail_quality: 80,
            strip_metadata: true,
            ffmpeg_path: None,
        }
    }
}

/// Media processing service.
#[derive(Clone)]
pub struct MediaService {
    config: MediaConfig,
}

impl MediaService {
    /// Create a new media service.
    #[must_use]
    pub const fn new(config: MediaConfig) -> Self {
        Self { config }
    }

    /// Get image metadata from file data.
    pub fn get_image_metadata(&self, data: &[u8]) -> AppResult<ImageMetadata> {
        // In a real implementation, use image-rs to extract metadata
        // This is a placeholder implementation

        // Detect format from magic bytes
        let format = self.detect_image_format(data)?;

        // Get dimensions (placeholder - would use image-rs)
        let dimensions = ImageDimensions {
            width: 0,
            height: 0,
        };

        Ok(ImageMetadata {
            dimensions,
            format,
            file_size: data.len() as u64,
            has_alpha: matches!(
                format,
                ImageFormat::Png | ImageFormat::WebP | ImageFormat::Gif
            ),
            is_animated: matches!(format, ImageFormat::Gif),
            blurhash: None,
            dominant_color: None,
            exif: None,
        })
    }

    /// Detect image format from magic bytes.
    fn detect_image_format(&self, data: &[u8]) -> AppResult<ImageFormat> {
        if data.len() < 12 {
            return Err(AppError::Validation(
                "Data too short to detect format".to_string(),
            ));
        }

        // JPEG: FF D8 FF
        if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return Ok(ImageFormat::Jpeg);
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data[0..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
            return Ok(ImageFormat::Png);
        }

        // GIF: GIF87a or GIF89a
        if data[0..3] == [0x47, 0x49, 0x46] {
            return Ok(ImageFormat::Gif);
        }

        // WebP: RIFF....WEBP
        if data[0..4] == [0x52, 0x49, 0x46, 0x46] && data[8..12] == [0x57, 0x45, 0x42, 0x50] {
            return Ok(ImageFormat::WebP);
        }

        // AVIF: ftyp....avif or ftyp....mif1
        if data[4..8] == [0x66, 0x74, 0x79, 0x70]
            && (data[8..12] == [0x61, 0x76, 0x69, 0x66] || data[8..12] == [0x6D, 0x69, 0x66, 0x31])
        {
            return Ok(ImageFormat::Avif);
        }

        Err(AppError::Validation(
            "Unknown or unsupported image format".to_string(),
        ))
    }

    /// Generate thumbnail from image data.
    pub fn generate_thumbnail(
        &self,
        data: &[u8],
        size: ThumbnailSize,
    ) -> AppResult<ProcessedImage> {
        let format = self.detect_image_format(data)?;
        let (max_width, max_height) = size.dimensions();

        // In a real implementation, use image-rs to:
        // 1. Decode the image
        // 2. Resize maintaining aspect ratio
        // 3. Encode to WebP or original format

        // Placeholder implementation
        tracing::info!(
            format = ?format,
            max_width = max_width,
            max_height = max_height,
            "Would generate thumbnail (implementation pending)"
        );

        Ok(ProcessedImage {
            data: data.to_vec(), // Would be processed data
            format: if self.config.enable_webp_conversion {
                ImageFormat::WebP
            } else {
                format
            },
            dimensions: ImageDimensions {
                width: max_width,
                height: max_height,
            },
            file_size: data.len() as u64,
        })
    }

    /// Process and optimize an image.
    pub fn process_image(
        &self,
        data: &[u8],
        options: ImageProcessingOptions,
    ) -> AppResult<ProcessedImage> {
        let format = self.detect_image_format(data)?;
        let output_format = options.format.unwrap_or(format);

        // In a real implementation:
        // 1. Decode image
        // 2. Auto-orient if requested
        // 3. Resize if needed
        // 4. Strip metadata if requested
        // 5. Encode to target format with quality

        tracing::info!(
            input_format = ?format,
            output_format = ?output_format,
            quality = ?options.quality,
            "Would process image (implementation pending)"
        );

        Ok(ProcessedImage {
            data: data.to_vec(),
            format: output_format,
            dimensions: ImageDimensions {
                width: 0,
                height: 0,
            },
            file_size: data.len() as u64,
        })
    }

    /// Generate blurhash for an image.
    pub fn generate_blurhash(&self, data: &[u8]) -> AppResult<String> {
        // In a real implementation, use blurhash crate
        // Placeholder
        let _ = self.detect_image_format(data)?;

        tracing::info!("Would generate blurhash (implementation pending)");

        // Return a placeholder blurhash
        Ok("L00000fQfQfQfQfQfQfQfQfQfQfQ".to_string())
    }

    /// Extract video thumbnail at a specific time.
    pub async fn extract_video_thumbnail(
        &self,
        video_path: &Path,
        time_seconds: f64,
    ) -> AppResult<ProcessedImage> {
        let ffmpeg = self.config.ffmpeg_path.as_deref().unwrap_or("ffmpeg");

        // In a real implementation, call ffmpeg:
        // ffmpeg -ss {time} -i {input} -vframes 1 -f image2pipe -vcodec png -

        tracing::info!(
            path = %video_path.display(),
            time = time_seconds,
            ffmpeg = ffmpeg,
            "Would extract video thumbnail (implementation pending)"
        );

        Ok(ProcessedImage {
            data: vec![],
            format: ImageFormat::Jpeg,
            dimensions: ImageDimensions {
                width: 0,
                height: 0,
            },
            file_size: 0,
        })
    }

    /// Get video metadata using ffprobe.
    pub async fn get_video_metadata(&self, video_path: &Path) -> AppResult<VideoMetadata> {
        // In a real implementation, call ffprobe:
        // ffprobe -v quiet -print_format json -show_format -show_streams {input}

        tracing::info!(
            path = %video_path.display(),
            "Would get video metadata (implementation pending)"
        );

        Ok(VideoMetadata {
            duration: 0.0,
            width: 0,
            height: 0,
            frame_rate: None,
            video_codec: None,
            audio_codec: None,
            bitrate: None,
            file_size: 0,
        })
    }

    /// Convert GIF to WebM/MP4 for better compression.
    pub async fn convert_gif_to_video(
        &self,
        gif_data: &[u8],
        output_format: &str,
    ) -> AppResult<Vec<u8>> {
        // In a real implementation, use ffmpeg:
        // ffmpeg -f gif -i pipe:0 -c:v libvpx-vp9 -pix_fmt yuv420p -f webm pipe:1

        tracing::info!(
            output_format = output_format,
            input_size = gif_data.len(),
            "Would convert GIF to video (implementation pending)"
        );

        Ok(gif_data.to_vec())
    }

    /// Convert video to HLS/DASH for adaptive streaming.
    pub async fn transcode_for_streaming(
        &self,
        video_path: &Path,
        output_dir: &Path,
    ) -> AppResult<()> {
        // In a real implementation, use ffmpeg to create:
        // - Multiple quality levels
        // - HLS playlist (.m3u8) or DASH manifest (.mpd)

        tracing::info!(
            input = %video_path.display(),
            output = %output_dir.display(),
            "Would transcode for streaming (implementation pending)"
        );

        Ok(())
    }

    /// Check if a file type is supported.
    #[must_use]
    pub fn is_supported_image(&self, mime_type: &str) -> bool {
        ImageFormat::from_mime_type(mime_type).is_some()
    }

    /// Check if video transcoding is available.
    #[must_use]
    pub const fn is_video_transcoding_available(&self) -> bool {
        self.config.enable_video_transcoding && self.config.ffmpeg_path.is_some()
    }
}

/// Media service status response.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStatusResponse {
    /// Supported image formats
    pub supported_image_formats: Vec<ImageFormat>,
    /// WebP conversion enabled
    pub webp_conversion: bool,
    /// AVIF conversion enabled
    pub avif_conversion: bool,
    /// Video transcoding available
    pub video_transcoding: bool,
    /// Maximum image dimension
    pub max_image_dimension: u32,
    /// Maximum video duration (seconds)
    pub max_video_duration: u32,
}

impl MediaService {
    /// Get service status.
    #[must_use]
    pub fn status(&self) -> MediaStatusResponse {
        MediaStatusResponse {
            supported_image_formats: vec![
                ImageFormat::Jpeg,
                ImageFormat::Png,
                ImageFormat::Gif,
                ImageFormat::WebP,
            ],
            webp_conversion: self.config.enable_webp_conversion,
            avif_conversion: self.config.enable_avif_conversion,
            video_transcoding: self.is_video_transcoding_available(),
            max_image_dimension: self.config.max_image_dimension,
            max_video_duration: self.config.max_video_duration,
        }
    }
}
