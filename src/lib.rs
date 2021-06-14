use glow::*;
use std::sync::Arc;

pub enum CornerType {
    Round,
    Hard
}

pub trait Uniforms {
    unsafe fn set_uniforms (&self, gl: &Context, program: u32);
}

pub struct TransformUniforms {
    transform: cgmath::Matrix4<f32>
}

impl TransformUniforms {
    pub fn new () -> Self {
        use cgmath::SquareMatrix;
        Self {
            transform: cgmath::Matrix4::identity()
        }
    }

    pub fn translate (&mut self, x: f32, y: f32) {
        self.transform = self.transform * cgmath::Matrix4::from_translation(cgmath::vec3(x, y, 0.));
    }

    pub fn rotate (&mut self, x: f32) {
        self.transform = self.transform * cgmath::Matrix4::from_angle_z(cgmath::Rad(x));
    }
}

impl Uniforms for TransformUniforms {
    unsafe fn set_uniforms(&self, gl: &Context, program: u32) {
        let location = gl.get_uniform_location(program, "transform").unwrap();
        let data: &[f32; 16] = self.transform.as_ref();
        gl.uniform_matrix_4_f32_slice(Some(&location), false, data);
    }
}

pub struct ColorUniforms {
    color: [f32; 3]
}

impl Uniforms for ColorUniforms {
    unsafe fn set_uniforms(&self, gl: &Context, program: u32) {
        let location = gl.get_uniform_location(program, "ucolor").unwrap();
        gl.uniform_3_f32_slice(Some(&location), &self.color);
    }
}

impl ColorUniforms {
    pub fn new (r: f32, g: f32, b: f32,) -> Self {
        Self {
            color: [r, g, b]
        }
    }
    
    pub fn new_from_8 (r: u8, g: u8, b: u8,) -> Self {
        Self {
            color: [r as f32 / 255., g as f32 / 255., b as f32 / 255.]
        }
    }
}

pub struct ProjectionUniforms {
    projection: cgmath::Matrix4<f32>
}

impl ProjectionUniforms {
    pub fn new (size: (u32, u32)) -> Self {
        Self {
            projection: cgmath::ortho(0., size.0 as f32, size.1 as f32, 0., 0., 1.)
        }
    }
}

impl Uniforms for ProjectionUniforms {
    unsafe fn set_uniforms(&self, gl: &Context, program: u32) {
        let location = gl.get_uniform_location(program, "projection").unwrap();
        let data: &[f32; 16] = self.projection.as_ref();
        gl.uniform_matrix_4_f32_slice(Some(&location), false, data);
    }
}

pub struct Circle {
    vertex_array: u32,
    vertex_buffer: u32,
    index_buffer: u32,
    indices: usize,
    pub radius: f32,
    gl: Arc<Context>
}

impl Drop for Circle {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_buffer(self.vertex_buffer);
            self.gl.delete_buffer(self.index_buffer);
        }
    }
}

impl Circle {
    pub unsafe fn new (gl: Arc<Context>, radius: f32) -> Result<Self, lyon::tessellation::TessellationError> {
        use lyon::math::Point;
        use lyon::path::{builder::*, Winding};
        use lyon::tessellation::{FillTessellator, FillOptions, VertexBuffers};
        use lyon::tessellation::geometry_builder::simple_builder;

        let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
        let mut geometry_builder = simple_builder(&mut geometry);
        let options = FillOptions::tolerance(0.1);
        let mut tessellator = FillTessellator::new();
    
        let mut builder = tessellator.builder(
            &options,
            &mut geometry_builder,
        );
    
        builder.add_circle(
            Point::new(0., 0.),
            radius,
            Winding::Positive
        );
    
        builder.build()?;

        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));
    
        let vertex_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));

        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
    
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
    
        let mut vertices = Vec::new();
        for vertex in geometry.vertices {
            vertices.push(vertex.x,);
            vertices.push(vertex.y);
        }
    
        let mut vertex_buffer_data = Vec::<u8>::with_capacity(vertices.len() * 4);
        for float in vertices.iter() {
            vertex_buffer_data.extend_from_slice(&float.to_le_bytes());
        }

    
        let mut index_buffer_data = Vec::<u8>::with_capacity(geometry.indices.len() * 2);
        for n in geometry.indices.iter() {
            index_buffer_data.extend_from_slice(&n.to_le_bytes());
        }
    
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            vertex_buffer_data.as_ref(),
            glow::DYNAMIC_DRAW,
        );

        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            index_buffer_data.as_ref(),
            glow::DYNAMIC_DRAW,
        );

        Ok(Self {
            vertex_array,
            vertex_buffer,
            index_buffer,
            indices: geometry.indices.len(),
            radius,
            gl
        })
    }

    pub fn draw_with(&self, program: u32, position: cgmath::Vector2<f32>, color: ColorUniforms, resolution: (u32, u32)) {
        let mut uniforms: Vec<Box<dyn Uniforms>> = Vec::new();
        uniforms.push(Box::new(ProjectionUniforms::new(resolution)));
        uniforms.push(Box::new({
            let mut t = TransformUniforms::new();
            t.translate(position.x, position.y);
            t
        }));
        uniforms.push(Box::new(color));
        unsafe { self.render(program, uniforms) }
    }
}

impl GLObject for Circle {
    unsafe fn render(&self, program: u32, uniforms: Vec<Box<dyn Uniforms>>) {
        self.gl.use_program(Some(program));
        self.gl.bind_vertex_array(Some(self.vertex_array));
        self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));

        self.gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        for uniform in uniforms {
            uniform.set_uniforms(&self.gl, program); // set up all the uniforms for our shader
        }
        self.gl.draw_elements(TRIANGLES, self.indices as i32, UNSIGNED_SHORT, 0);
    }
}

//
#[derive(Debug, Clone)]
pub struct Rectangle {
    vertex_array: u32,
    vertex_buffer: u32,
    index_buffer: u32,
    indices: usize,
    pub width: f32,
    pub height: f32, 
    gl: Arc<Context>
}

impl Drop for Rectangle {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_buffer(self.vertex_buffer);
            self.gl.delete_buffer(self.index_buffer);
        }
    }
}

impl Rectangle {
    pub unsafe fn new (gl: Arc<Context>, width: f32, height: f32, kind: CornerType) -> Self {
        use lyon::math::{rect, Point};
        use lyon::path::{builder::*, Winding};
        use lyon::tessellation::{FillTessellator, FillOptions, VertexBuffers};
        use lyon::tessellation::geometry_builder::simple_builder;

        let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
        let mut geometry_builder = simple_builder(&mut geometry);
        let options = FillOptions::tolerance(0.1);
        let mut tessellator = FillTessellator::new();
    
        let mut builder = tessellator.builder(
            &options,
            &mut geometry_builder,
        );

        match kind {
            CornerType::Hard => builder.add_rectangle(
                &rect(0.0, 0.0, width, height),
                Winding::Positive
            ),
            CornerType::Round => builder.add_rounded_rectangle(
                &rect(0.0, 0.0, width, height),
                &BorderRadii {
                    top_left: 4.5,
                    top_right: 4.5,
                    bottom_left: 4.5,
                    bottom_right: 4.5,
                },
                Winding::Positive
            )
        }
    
        builder.build().unwrap();

        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));
    
        let vertex_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));

        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
    
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
    
        let mut vertices = Vec::new();
        for vertex in geometry.vertices {
            vertices.push(vertex.x,);
            vertices.push(vertex.y);
        }
    
        let mut vertex_buffer_data = Vec::<u8>::with_capacity(vertices.len() * 4);
        for float in vertices.iter() {
            vertex_buffer_data.extend_from_slice(&float.to_le_bytes());
        }

    
        let mut index_buffer_data = Vec::<u8>::with_capacity(geometry.indices.len() * 2);
        for n in geometry.indices.iter() {
            index_buffer_data.extend_from_slice(&n.to_le_bytes());
        }
    
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            vertex_buffer_data.as_ref(),
            glow::STATIC_DRAW,
        );

        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            index_buffer_data.as_ref(),
            glow::STATIC_DRAW,
        );


        Self {
            vertex_array,
            vertex_buffer,
            index_buffer,
            indices: geometry.indices.len(),
            width,
            height,
            gl
        }
    }

    pub unsafe fn update (&mut self, width: f32, height: f32, kind: CornerType) {
        use lyon::math::{rect, Point};
        use lyon::path::{builder::*, Winding};
        use lyon::tessellation::{FillTessellator, FillOptions, VertexBuffers};
        use lyon::tessellation::geometry_builder::simple_builder;

        let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
        let mut geometry_builder = simple_builder(&mut geometry);
        let options = FillOptions::tolerance(0.1);
        let mut tessellator = FillTessellator::new();
    
        let mut builder = tessellator.builder(
            &options,
            &mut geometry_builder,
        );
    
        match kind {
            CornerType::Hard => builder.add_rectangle(
                &rect(0.0, 0.0, width, height),
                Winding::Positive
            ),
            CornerType::Round => builder.add_rounded_rectangle(
                &rect(0.0, 0.0, width, height),
                &BorderRadii {
                    top_left: 4.5,
                    top_right: 4.5,
                    bottom_left: 4.5,
                    bottom_right: 4.5,
                },
                Winding::Positive
            )
        }
    
        builder.build().unwrap();

        self.gl.bind_vertex_array(Some(self.vertex_array));
        self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertex_buffer));
        self.gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
    
        self.gl.enable_vertex_attrib_array(0);
        self.gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
    
        let mut vertices = Vec::new();
        for vertex in geometry.vertices {
            vertices.push(vertex.x,);
            vertices.push(vertex.y);
        }
    
        let mut vertex_buffer_data = Vec::<u8>::with_capacity(vertices.len() * 4);
        for float in vertices.iter() {
            vertex_buffer_data.extend_from_slice(&float.to_le_bytes());
        }

    
        let mut index_buffer_data = Vec::<u8>::with_capacity(geometry.indices.len() * 2);
        for n in geometry.indices.iter() {
            index_buffer_data.extend_from_slice(&n.to_le_bytes());
        }
    
        self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            vertex_buffer_data.as_ref(),
            glow::STATIC_DRAW,
        );

        self.gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            index_buffer_data.as_ref(),
            glow::STATIC_DRAW,
        );

        self.width = width;
        self.height = height;
    }

    pub fn draw_with(&self, program: u32, position: cgmath::Vector2<f32>, color: ColorUniforms, resolution: (u32, u32)) {
        let mut uniforms: Vec<Box<dyn Uniforms>> = Vec::new();
        uniforms.push(Box::new(ProjectionUniforms::new(resolution)));
        uniforms.push(Box::new({
            let mut t = TransformUniforms::new();
            t.translate(position.x, position.y);
            t
        }));
        uniforms.push(Box::new(color));
        unsafe { self.render(program, uniforms) }
    }
}

impl GLObject for Rectangle {
    unsafe fn render(&self, program: u32, uniforms: Vec<Box<dyn Uniforms>>) {
        self.gl.use_program(Some(program));
        self.gl.bind_vertex_array(Some(self.vertex_array));
        self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));

        self.gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        for uniform in uniforms {
            uniform.set_uniforms(&self.gl, program); // set up all the uniforms for our shader
        }
        self.gl.draw_elements(TRIANGLES, self.indices as i32, UNSIGNED_SHORT, 0);
    }
}
//

pub struct RadialGradient {
    vertex_array: u32,
    vertex_buffer: u32,
    index_buffer: u32,
    indices: usize,
    pub radius: f32,
    gl: Arc<Context>
}

impl Drop for RadialGradient {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_buffer(self.vertex_buffer);
            self.gl.delete_buffer(self.index_buffer);
        }
    }
}

impl RadialGradient {
    pub unsafe fn new (gl: Arc<Context>, radius: f32) -> Result<Self, lyon::tessellation::TessellationError> {
        use lyon::math::Point;
        use lyon::path::{builder::*, Winding};
        use lyon::tessellation::{FillTessellator, FillOptions, VertexBuffers};
        use lyon::tessellation::geometry_builder::simple_builder;

        let mut geometry: VertexBuffers<Point, u16> = VertexBuffers::new();
        let mut geometry_builder = simple_builder(&mut geometry);
        let options = FillOptions::tolerance(0.1);
        let mut tessellator = FillTessellator::new();
    
        let mut builder = tessellator.builder(
            &options,
            &mut geometry_builder,
        );
    
        builder.add_circle(
            Point::new(0., 0.),
            radius,
            Winding::Positive
        );
    
        builder.build()?;

        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));
    
        let vertex_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer));

        let index_buffer = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer));
    
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
    
        let mut vertices = Vec::new();
        for vertex in geometry.vertices {
            vertices.push(vertex.x,);
            vertices.push(vertex.y);
        }
    
        let mut vertex_buffer_data = Vec::<u8>::with_capacity(vertices.len() * 4);
        for float in vertices.iter() {
            vertex_buffer_data.extend_from_slice(&float.to_le_bytes());
        }

    
        let mut index_buffer_data = Vec::<u8>::with_capacity(geometry.indices.len() * 2);
        for n in geometry.indices.iter() {
            index_buffer_data.extend_from_slice(&n.to_le_bytes());
        }
    
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            vertex_buffer_data.as_ref(),
            glow::DYNAMIC_DRAW,
        );

        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            index_buffer_data.as_ref(),
            glow::DYNAMIC_DRAW,
        );

        Ok(Self {
            vertex_array,
            vertex_buffer,
            index_buffer,
            indices: geometry.indices.len(),
            radius,
            gl
        })
    }

    pub fn draw_with(&self, program: u32, position: cgmath::Vector2<f32>, color: ColorUniforms, resolution: (u32, u32)) {
        let mut uniforms: Vec<Box<dyn Uniforms>> = Vec::new();
        uniforms.push(Box::new(ProjectionUniforms::new(resolution)));
        uniforms.push(Box::new({
            let mut t = TransformUniforms::new();
            t.translate(position.x, position.y);
            t
        }));
        uniforms.push(Box::new(color));
        uniforms.push(Box::new(GenericVec2Uniform::new(String::from("center"), position)));
        uniforms.push(Box::new(GenericFloatUniform::new(String::from("range"), self.radius)));
        unsafe { self.render(program, uniforms) }
    }
}

impl GLObject for RadialGradient {
    unsafe fn render(&self, program: u32, uniforms: Vec<Box<dyn Uniforms>>) {
        self.gl.use_program(Some(program));
        self.gl.bind_vertex_array(Some(self.vertex_array));
        self.gl.bind_buffer(ARRAY_BUFFER, Some(self.vertex_buffer));

        self.gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(self.index_buffer));
        for uniform in uniforms {
            uniform.set_uniforms(&self.gl, program); // set up all the uniforms for our shader
        }
        self.gl.draw_elements(TRIANGLES, self.indices as i32, UNSIGNED_SHORT, 0);
    }
}

pub struct GenericVec2Uniform {
    name: String,
    value: cgmath::Vector2<f32>
}

impl GenericVec2Uniform {
    pub fn new (name: String, value: cgmath::Vector2<f32>) -> Self {
        Self {
            name,
            value
        }
    }
}

impl Uniforms for GenericVec2Uniform {
    unsafe fn set_uniforms(&self, gl: &Context, program: u32) {
        let location = gl.get_uniform_location(program, self.name.as_str()).unwrap();
        gl.uniform_2_f32(Some(&location), self.value.x, self.value.y);
    }
}

pub struct GenericFloatUniform {
    name: String,
    value: f32
}

impl GenericFloatUniform {
    fn new (name: String, value: f32) -> Self {
        Self {
            name,
            value
        }
    }
}

impl Uniforms for GenericFloatUniform {
    unsafe fn set_uniforms(&self, gl: &Context, program: u32) {
        let location = gl.get_uniform_location(program, self.name.as_str()).unwrap();
        gl.uniform_1_f32(Some(&location), self.value);
    }
}

pub trait GLObject {
    unsafe fn render(&self, program: u32, uniforms: Vec<Box<dyn Uniforms>>);
}

pub unsafe fn set_clear_color (gl: &Context, color: ColorUniforms) {
    gl.clear_color(color.color[0], color.color[1], color.color[2], 0.);
}

pub fn compile_shader (gl: &glow::Context, vertex_shader_source: &str, fragment_shader_source: &str) -> u32 {
    unsafe {
        let program = gl.create_program().expect("Cannot create program"); // compile and link shader program

        let shader_sources = [
            (glow::VERTEX_SHADER, vertex_shader_source),
            (glow::FRAGMENT_SHADER, fragment_shader_source),
        ];
    
        let mut shaders = Vec::with_capacity(shader_sources.len());
    
        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
            gl.shader_source(shader, &format!("{}\n{}", "#version 330", shader_source));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                std::panic::panic_any(gl.get_shader_info_log(shader));
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
    
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            std::panic::panic_any(gl.get_program_info_log(program));
        }
    
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        program
    }
}

pub struct OutlinedCircle {
    pub outline: Circle,
    pub inner: Circle
}