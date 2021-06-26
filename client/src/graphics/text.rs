use rusttype::{gpu_cache::Cache, point, Font, PositionedGlyph, Rect, Scale};

use std::sync::Arc;
use vulkano::{
  buffer::{BufferUsage, CpuAccessibleBuffer},
  command_buffer::{
    AutoCommandBufferBuilder, DynamicState, PrimaryAutoCommandBuffer, SubpassContents,
  },
  descriptor::{descriptor_set::PersistentDescriptorSet, pipeline_layout::PipelineLayoutAbstract},
  device::{Device, Queue},
  format::{ClearValue, Format},
  image::{
    view::ImageView, ImageCreateFlags, ImageDimensions, ImageLayout, ImageUsage, ImmutableImage,
    SwapchainImage,
  },
  pipeline::{vertex::SingleBufferDefinition, GraphicsPipeline},
  render_pass::{Framebuffer, FramebufferAbstract, RenderPass, Subpass},
  sampler::{Filter, MipmapMode, Sampler, SamplerAddressMode},
  swapchain::Swapchain,
};
use winit::window::Window;

#[derive(Default, Debug, Clone)]
struct Vert {
  pos: [f32; 2],
  uv:  [f32; 2],
  col: [f32; 4],
}
vulkano::impl_vertex!(Vert, pos, uv, col);

mod vs {
  vulkano_shaders::shader! {
    ty: "vertex",
    path: "src/shader/text.vs",
  }
}

mod fs {
  vulkano_shaders::shader! {
    ty: "fragment",
    path: "src/shader/text.fs",
  }
}

struct TextData {
  glyphs: Vec<PositionedGlyph<'static>>,
  color:  [f32; 4],
}

pub struct TextRender {
  device:             Arc<Device>,
  queue:              Arc<Queue>,
  font:               Font<'static>,
  cache:              Cache<'static>,
  cache_pixel_buffer: Vec<u8>,
  pipeline: Arc<
    GraphicsPipeline<SingleBufferDefinition<Vert>, Box<dyn PipelineLayoutAbstract + Send + Sync>>,
  >,
  texts:              Vec<TextData>,
}

const CACHE_WIDTH: usize = 1000;
const CACHE_HEIGHT: usize = 1000;

impl TextRender {
  pub fn new<W>(device: Arc<Device>, queue: Arc<Queue>, swapchain: Arc<Swapchain<W>>) -> Self
  where
    W: Send + Sync + 'static,
  {
    let font_data = include_bytes!("/usr/share/fonts/TTF/DejaVuSans.ttf");
    let font = Font::try_from_bytes(font_data as &[u8]).unwrap();

    let vs = vs::Shader::load(device.clone()).unwrap();
    let fs = fs::Shader::load(device.clone()).unwrap();

    let cache = Cache::builder().dimensions(CACHE_WIDTH as u32, CACHE_HEIGHT as u32).build();
    let cache_pixel_buffer = vec![0; CACHE_WIDTH * CACHE_HEIGHT];

    let render_pass = Arc::new(
      vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
          color: {
            load: Load,
            store: Store,
            format: swapchain.format(),
            samples: 1,
          }
        },
        pass: {
          color: [color],
          depth_stencil: {}
        }
      )
      .unwrap(),
    ) as Arc<RenderPass>;

    let pipeline = Arc::new(
      GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vert>()
        .vertex_shader(vs.main_entry_point(), ())
        .triangle_list()
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(fs.main_entry_point(), ())
        .blend_alpha_blending()
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap(),
    );

    TextRender { device, queue, font, cache, cache_pixel_buffer, pipeline, texts: vec![] }
  }

  pub fn queue_text(&mut self, x: f32, y: f32, size: f32, color: [f32; 4], text: &str) {
    let glyphs: Vec<PositionedGlyph> =
      self.font.layout(text, Scale::uniform(size), point(x, y)).map(|g| g.clone()).collect();
    for glyph in &glyphs.clone() {
      self.cache.queue_glyph(0, glyph.clone());
    }
    self.texts.push(TextData { glyphs: glyphs.clone(), color });
  }

  pub fn draw_text<'a>(
    &mut self,
    mut command_buffer: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    framebuf: Arc<Framebuffer<((), Arc<ImageView<Arc<SwapchainImage<Window>>>>)>>,
  ) -> &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
    let screen_width = framebuf.dimensions()[0];
    let screen_height = framebuf.dimensions()[1];
    let cache_pixel_buffer = &mut self.cache_pixel_buffer;
    let cache = &mut self.cache;

    // update texture cache
    cache
      .cache_queued(|rect, src_data| {
        let width = (rect.max.x - rect.min.x) as usize;
        let height = (rect.max.y - rect.min.y) as usize;
        let mut dst_index = rect.min.y as usize * CACHE_WIDTH + rect.min.x as usize;
        let mut src_index = 0;

        for _ in 0..height {
          let dst_slice = &mut cache_pixel_buffer[dst_index..dst_index + width];
          let src_slice = &src_data[src_index..src_index + width];
          dst_slice.copy_from_slice(src_slice);

          dst_index += CACHE_WIDTH;
          src_index += width;
        }
      })
      .unwrap();

    let buffer = CpuAccessibleBuffer::<[u8]>::from_iter(
      self.device.clone(),
      BufferUsage::all(),
      false,
      cache_pixel_buffer.iter().cloned(),
    )
    .unwrap();

    let (cache_texture, cache_texture_write) = ImmutableImage::uninitialized(
      self.device.clone(),
      ImageDimensions::Dim2d {
        width:        CACHE_WIDTH as u32,
        height:       CACHE_HEIGHT as u32,
        array_layers: 1,
      },
      Format::R8Unorm,
      1,
      ImageUsage { sampled: true, transfer_destination: true, ..ImageUsage::none() },
      ImageCreateFlags::none(),
      ImageLayout::General,
      Some(self.queue.family()),
    )
    .unwrap();

    let sampler = Sampler::new(
      self.device.clone(),
      Filter::Linear,
      Filter::Linear,
      MipmapMode::Nearest,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      SamplerAddressMode::Repeat,
      0.0,
      1.0,
      0.0,
      0.0,
    )
    .unwrap();

    let cache_texture_view = ImageView::new(cache_texture).unwrap();

    let set = Arc::new(
      PersistentDescriptorSet::start(self.pipeline.descriptor_set_layout(0).unwrap().clone())
        .add_sampled_image(cache_texture_view, sampler)
        .unwrap()
        .build()
        .unwrap(),
    );

    // let mut command_buffer = command_buffer
    //   .copy_buffer_to_image(buffer, cache_texture_write)
    //   .unwrap()
    //   .begin_render_pass(
    //     framebuf.clone(),
    //     SubpassContents::Inline,
    //     vec![ClearValue::Float([0.0, 0.0, 0.2, 1.0])],
    //   )
    //   .unwrap();

    // draw
    for text in &mut self.texts.drain(..) {
      let vertices: Vec<Vert> = text
        .glyphs
        .iter()
        .flat_map(|g| {
          if let Ok(Some((uv_rect, screen_rect))) = cache.rect_for(0, g) {
            let gl_rect = Rect {
              min: point(
                (screen_rect.min.x as f32 / screen_width as f32 - 0.5) * 2.0,
                (screen_rect.min.y as f32 / screen_height as f32 - 0.5) * 2.0,
              ),
              max: point(
                (screen_rect.max.x as f32 / screen_width as f32 - 0.5) * 2.0,
                (screen_rect.max.y as f32 / screen_height as f32 - 0.5) * 2.0,
              ),
            };
            vec![
              Vert {
                pos: [gl_rect.min.x, gl_rect.max.y],
                uv:  [uv_rect.min.x, uv_rect.max.y],
                col: text.color,
              },
              Vert {
                pos: [gl_rect.min.x, gl_rect.min.y],
                uv:  [uv_rect.min.x, uv_rect.min.y],
                col: text.color,
              },
              Vert {
                pos: [gl_rect.max.x, gl_rect.min.y],
                uv:  [uv_rect.max.x, uv_rect.min.y],
                col: text.color,
              },
              Vert {
                pos: [gl_rect.max.x, gl_rect.min.y],
                uv:  [uv_rect.max.x, uv_rect.min.y],
                col: text.color,
              },
              Vert {
                pos: [gl_rect.max.x, gl_rect.max.y],
                uv:  [uv_rect.max.x, uv_rect.max.y],
                col: text.color,
              },
              Vert {
                pos: [gl_rect.min.x, gl_rect.max.y],
                uv:  [uv_rect.min.x, uv_rect.max.y],
                col: text.color,
              },
            ]
            .into_iter()
          } else {
            vec![].into_iter()
          }
        })
        .collect();

      let vertex_buffer = CpuAccessibleBuffer::from_iter(
        self.device.clone(),
        BufferUsage::all(),
        false,
        vertices.into_iter(),
      )
      .unwrap();
      command_buffer = command_buffer
        .draw(
          self.pipeline.clone(),
          &DynamicState::none(),
          vertex_buffer.clone(),
          set.clone(),
          (),
          vec![],
        )
        .unwrap();
    }

    // command_buffer.end_render_pass().unwrap()
    command_buffer
  }
}
