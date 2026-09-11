use std::path::PathBuf;

#[derive(PartialEq)]
pub struct FileParserResult {
    pub vpath: String,
    pub caller: PathBuf,
    pub resolver_type: PathResolverType,
    pub format_define: FileFormatDefine,
}

impl FileParserResult {
    pub fn new(
        vpath: String,
        caller: PathBuf,
        resolver_type: PathResolverType,
        format_define: FileFormatDefine,
    ) -> Self {
        Self {
            vpath,
            caller,
            resolver_type,
            format_define,
        }
    }
}

pub struct PreResolveAssetData {
    pub vpath: String,
    pub format_define: FileFormatDefine,
}

impl PreResolveAssetData {
    pub fn new(vpath: String, format_define: FileFormatDefine) -> Self {
        Self {
            vpath,
            format_define,
        }
    }
}

#[derive(PartialEq)]
pub enum PathResolverType {
    Pack,
    Standalone,
}

#[derive(PartialEq, Clone, Copy)]
pub enum FileFormatDefine {
    Signature,
    Extension,
}
