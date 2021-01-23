use image::{DynamicImage, ImageBuffer, Luma, Pixel, Primitive, Rgb};
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Args {
    #[structopt(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,

    #[structopt(short = "g", long)]
    to_gray: bool,

    #[structopt(default_value = "100", short = "a", long)]
    alpha: u8,
}

const DPI: f64 = 300.0;

fn open_image(path: &Path) -> image::ImageResult<image::DynamicImage> {
    image::io::Reader::open(path)?.decode()
}

trait MulAlpha
where
    Self: Pixel + 'static,
    Self::Subpixel: Primitive + 'static,
{
    fn mul_alpha(&self, alpha: f32) -> Self;

    fn mul_alpha_buffer(
        img: &ImageBuffer<Self, Vec<Self::Subpixel>>,
        alpha: f32,
    ) -> ImageBuffer<Self, Vec<Self::Subpixel>> {
        let (width, height) = img.dimensions();
        let mut out = ImageBuffer::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y).mul_alpha(alpha);
                out.put_pixel(x, y, pixel);
            }
        }

        out
    }
}

impl<S: Primitive + std::fmt::Debug + 'static> MulAlpha for Luma<S> {
    fn mul_alpha(&self, alpha: f32) -> Self {
        self.map_with_alpha(
            |p| {
                let max_pixel: f32 = num_traits::NumCast::from(S::max_value()).unwrap();
                let bgrnd: f32 = (1.0 - alpha) * max_pixel;
                let p_as_f32: f32 = num_traits::NumCast::from(p).unwrap();
                let fgrnd: f32 = alpha * p_as_f32;
                num_traits::NumCast::from(bgrnd + fgrnd).unwrap()
            },
            |_| S::max_value(),
        )
    }
}

impl<S: Primitive + std::fmt::Debug + 'static> MulAlpha for Rgb<S> {
    fn mul_alpha(&self, alpha: f32) -> Self {
        self.map_with_alpha(
            |p| {
                let max_pixel: f32 = num_traits::NumCast::from(S::max_value()).unwrap();
                let bgrnd: f32 = (1.0 - alpha) * max_pixel;
                let p_as_f32: f32 = num_traits::NumCast::from(p).unwrap();
                let fgrnd: f32 = alpha * p_as_f32;
                num_traits::NumCast::from(bgrnd + fgrnd).unwrap()
            },
            |_| S::max_value(),
        )
    }
}

fn mul_alpha_to_image(img: &DynamicImage, alpha: f32) -> DynamicImage {
    match img {
        DynamicImage::ImageLuma8(buffer) => {
            DynamicImage::ImageLuma8(MulAlpha::mul_alpha_buffer(buffer, alpha))
        }
        DynamicImage::ImageRgb8(buffer) => {
            DynamicImage::ImageRgb8(MulAlpha::mul_alpha_buffer(buffer, alpha))
        }
        DynamicImage::ImageRgb16(buffer) => {
            DynamicImage::ImageRgb16(MulAlpha::mul_alpha_buffer(buffer, alpha))
        }
        _ => unimplemented!("add_alpha_to_image"),
    }
}

fn process_image(args: &Args, img: image::DynamicImage) -> image::DynamicImage {
    let mut output = img;

    if args.to_gray {
        let temp = image::DynamicImage::ImageLuma8(image::imageops::grayscale(&output));
        output = temp;
    }

    if args.alpha < 100 {
        let alpha = f32::from(args.alpha) / 100.0;
        let temp = mul_alpha_to_image(&output, alpha);
        output = temp;
    }

    output
}

fn create_pdf(doc_name: &str, img_view: &image::DynamicImage) -> PdfDocumentReference {
    let pdf_image = Image::from_dynamic_image(img_view);
    let (doc, page, layer) = PdfDocument::new(
        doc_name,
        pdf_image.image.width.into_pt(DPI).into(),
        pdf_image.image.height.into_pt(DPI).into(),
        "Layer 1",
    );

    let current_layer = doc.get_page(page).get_layer(layer);
    pdf_image.add_to_layer(current_layer, None, None, None, None, None, Some(DPI));

    doc
}

fn main() -> std::result::Result<(), ()> {
    let args = Args::from_args();

    for file in &args.files {
        let image = open_image(&file).map_err(|_| ())?;
        let processed = process_image(&args, image);
        let pdf = create_pdf(&file.to_string_lossy(), &processed);

        let outfile = file.with_extension("pdf");
        pdf.save(&mut BufWriter::new(File::create(outfile).unwrap()))
            .unwrap();
    }

    Ok(())
}
