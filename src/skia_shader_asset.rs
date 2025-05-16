use std::ops::Deref;
use assets_manager::{loader::Loader, Asset};
use skia_safe::RuntimeEffect;

pub struct SkiaShaderAsset(pub RuntimeEffect);

// Shutup rustc, this shit is not even gonna leave the main thread bruh
unsafe impl Sync for SkiaShaderAsset {}
unsafe impl Send for SkiaShaderAsset {}

impl Deref for SkiaShaderAsset {
    type Target = RuntimeEffect;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


impl Asset for SkiaShaderAsset {
    const EXTENSIONS: &'static [&'static str] = &["sksl"];
    type Loader = SkiaShaderLoader;
}
impl From<RuntimeEffect> for SkiaShaderAsset {
    fn from(value: RuntimeEffect) -> Self {
        Self(value)
    }
}

pub struct SkiaShaderLoader;

impl Loader<SkiaShaderAsset> for SkiaShaderLoader {
    fn load(
        content: std::borrow::Cow<[u8]>,
        _ext: &str,
    ) -> Result<SkiaShaderAsset, assets_manager::BoxedError> {
        let source = std::str::from_utf8(&content)?;
        let effect = RuntimeEffect::make_for_shader(source, None)?;
        Ok(SkiaShaderAsset(effect))
    }
}
