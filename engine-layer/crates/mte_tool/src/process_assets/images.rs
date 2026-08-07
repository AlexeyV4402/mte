use std::io::Cursor;

use ctt::encoders::Encoder;
use ctt::{
    AlphaMode, ColorSpace, Container, ConvertSettings, Format, Image, Surface, TargetFormat, TextureKind, convert
};
use image::{GenericImageView, ImageReader};

pub(crate) fn process_png_jpg(input_content: &[u8]) -> Result<Vec<u8>, String> {
    // 1. Декодируем сжатый PNG/JPG из памяти в сырую RGBA картинку
    let dynamic_image = ImageReader::new(Cursor::new(input_content))
        .with_guessed_format()
        .map_err(|e| format!("Не удалось определить формат: {}", e))?
        .decode()
        .map_err(|e| format!("Ошибка декодирования: {}", e))?;

    let (width, height) = dynamic_image.dimensions();

    // 2. ЖЕСТКАЯ ПРОВЕРКА ДЛЯ BC7: Длина и ширина ОБЯЗАНЫ делиться на 4!
    // Для картинок вроде 364x498 тебе придется расширить их в фоторедакторе до 364x500
    // if width % 4 != 0 || height % 4 != 0 {
    //     return Err(format!(
    //         "Размер {}x{} не кратен 4; Сжатие BC7 невозможно.",
    //         width, height
    //     ));
    // }

    // Переводим в сырой вектор байт RGBA8 (теперь он будет весить честные width * height * 4)
    let raw_rgba_bytes = dynamic_image.to_rgba8().into_raw();

    // 3. Передаем СЫРЫЕ байты в структуру ctt
    let surface = Surface {
        data: raw_rgba_bytes, // Честные 725+ КБ данных
        width,
        height,
        depth: 1,
        stride: width * 4,
        slice_stride: 0,
        format: Format::R8G8B8A8_UNORM, // Теперь это правда
        color_space: ColorSpace::Srgb,
        alpha: AlphaMode::Straight,
    };

    let image = Image {
        surfaces: vec![vec![surface]],
        kind: TextureKind::Texture2D,
    };

    // 4. Запускаем конвертацию в KTX2 + BC7
    let output = match convert(
        image,
        ConvertSettings {
            format: Some(TargetFormat::Compressed {
                format: Format::BC7_UNORM_BLOCK,
                encoder: Encoder::Auto,
            }),
            container: Container::ktx2(),
            ..Default::default()
        },
    )
    .map_err(|err| format!("Ошибка конвертации: {}", err))?
    {
        ctt::PipelineOutput::Encoded(items) => items,
        ctt::PipelineOutput::Raw(_) => {
            return Err("Ошибка обработки файла".to_string());
        }
    };

    Ok(output)
}
