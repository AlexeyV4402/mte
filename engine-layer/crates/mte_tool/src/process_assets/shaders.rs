use naga::back::spv;
use naga::front::glsl;
use naga::{FastHashMap, valid};

pub fn process_glsl(input_content: Vec<u8>, ext: &String) -> Result<Vec<u8>, String> {
    let source_text: &str = std::str::from_utf8(&input_content)
        .map_err(|e| format!("Шейдер содержит невалидный UTF-8 код: {}", e))?;

    let options = glsl::Options {
        stage: match ext.as_str() {
            "vertex" => naga::ShaderStage::Vertex,
            "fragment" => naga::ShaderStage::Fragment,
            "compute" => naga::ShaderStage::Compute,
            _ => return Err("Неизвестное предназначение шейдера".to_string()),
        },
        defines: FastHashMap::default(),
    };

    let mut frontend = glsl::Frontend::default();

    let module = frontend
        .parse(&options, source_text)
        .map_err(|e| format!("Ошибка парсинга GLSL:\n{:?}", e))?;

    let mut validator =
        valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::all());

    let module_info = validator
        .validate(&module)
        .map_err(|e| format!("Шейдер не прошёл валидацию:\n{:?}", e))?;

    let mut writer_options = spv::Options::default();
    writer_options.flags = spv::WriterFlags::DEBUG;

    let spirv_words = spv::write_vec(&module, &module_info, &writer_options, None)
        .map_err(|e| format!("Ошибка генерации SPIR-V: {:?}", e))?;

    Ok(bytemuck::cast_slice(&spirv_words).to_vec())
}
