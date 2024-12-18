use eframe::egui::ColorImage;

pub fn image_from_path(path: &std::path::Path) -> Result<ColorImage, image::ImageError> {
	let image: image::DynamicImage = image::ImageReader::open(path)?.decode()?;
	Ok(ColorImage::from_rgba_unmultiplied(
		[image.width() as usize, image.height() as usize],
		image.to_rgba8().as_flat_samples().as_slice(),
	))
}