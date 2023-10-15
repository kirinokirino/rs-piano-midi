use glam::Vec2;

use std::f32::consts::PI;
use std::io::Write;
use std::process::{ChildStdin, Command, Stdio};

const FPS: f64 = 30.0;
const FRAME_TIME: f64 = 1.0 / FPS;
// seconds of future notes visible on screen.
const VIEW: f32 = 0.4;

const RECORD: bool = false;
const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const PALETTE: [&'static str; 5] = ["#160729", "#171856", "#243771", "#416e8f", "#dbf3f1"];
const SLOPE: f32 = 30.0;
const SLOPE_ANGLE: f32 = SLOPE / 480.0;

mod song;
use song::NOTES;

fn main() {
    let mut sketch = Sketch::new();
    sketch.run();
}

const GRAVITY: Vec2 = Vec2 { x: 0.0, y: 1.0 };
#[derive(Clone)]
struct Particle {
    pos: Vec2,
    vel: Vec2,

    lifetime: f32,
}

impl Particle {
    fn new(pos: Vec2, vel: Vec2) -> Self {
        Self {
            pos,
            vel,
            lifetime: 0.0,
        }
    }

    pub fn update(&mut self) {
        self.pos += self.vel;
        self.vel += GRAVITY;
        self.lifetime += FRAME_TIME as f32;
    }
}

struct Particles {
    particles: Vec<Particle>,
    lines: Vec<(Vec2, Vec2)>,
}

impl Particles {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        for particle in &mut self.particles {
            particle.update();
        }
        let new: Vec<Particle> = self
            .particles
            .iter()
            .cloned()
            .filter(|particle| !(particle.pos.y >= HEIGHT as f32))
            .collect();
        self.particles = new;
    }

    pub fn draw(&mut self, canvas: &mut Canvas) {
        canvas.select_color(2);
        for particle in &self.particles {
            let (pos, next_pos) = (particle.pos, particle.vel + particle.pos);
            let mut middle = (pos + next_pos) / 2.0;
            middle -= GRAVITY;
            canvas.draw_curve(pos, middle, next_pos);
        }
        for line in &self.lines {
            canvas.draw_line(line.0, line.1);
        }
        self.lines.clear()
    }

    fn particles_for_note(&mut self, pos: Vec2) {
        let rest_y = HEIGHT as f32 - pos.y;
        let end_x = rest_y * SLOPE_ANGLE + pos.x;
        let end = Vec2::new(end_x, HEIGHT as f32);
        self.lines.push((pos, end));
        self.spawn_explosion(end);
    }

    fn spawn_explosion(&mut self, pos: Vec2) {
        for i in 0..fastrand::usize(2..5) {
            let mut vel = Vec2::from_angle(-fastrand::f32() * PI);
            vel *= fastrand::f32() * 15.0;
            let particle = Particle::new(pos, vel);
            self.particles.push(particle);
        }
    }
}

struct Sketch {
    canvas: Canvas,
    ffmpeg: Option<ChildStdin>,

    frame: usize,
    time: f32,
    visible_notes: Vec<(f32, u8)>,
    note_lowest_highest: (u8, u8),
    droplets: Particles,
}

impl Sketch {
    pub fn new() -> Self {
        let ffmpeg = Self::ffmpeg();
        let canvas = Self::canvas();

        let note_lowest_highest = note_find_lowest_highest();
        Self {
            canvas,
            ffmpeg,
            frame: 0,
            time: 0f32,
            visible_notes: Vec::new(),
            note_lowest_highest,
            droplets: Particles::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            self.update();
            self.draw();
            std::thread::sleep(std::time::Duration::from_secs_f64(FRAME_TIME));
            self.frame += 1;
        }
    }

    fn update(&mut self) {
        self.droplets.update();
        self.update_visible_notes();
        for (time, note) in &self.visible_notes {
            let close_to_end = time - self.time < FRAME_TIME as f32;
            if close_to_end {
                let pos = self.pos_for(&(*time, *note));
                self.droplets.particles_for_note(pos);
            }
        }
    }

    fn draw(&mut self) {
        // self.canvas.blend_mode = BlendMode::Blend;
        // self.canvas.pen_color = hex_to_rgb(&PALETTE[PALETTE.len() - 1]);
        // self.canvas.pen_color[3] = 20;
        // self.canvas.draw_square(
        //     Vec2::new(0.0, 0.0),
        //     Vec2::new((WIDTH) as f32, HEIGHT as f32),
        // );
        // self.canvas.blend_mode = BlendMode::Replace;
        self.canvas.buffer.fill(0);
        // self.canvas.dim(10);
        //self.canvas.random();
        let (low, high) = self.note_lowest_highest;
        for note in &self.visible_notes {
            let palette = map(note.1 as f32, low as f32, high as f32, 0.0, 5.0).round() as u8;
            self.canvas.select_color(palette);
            let prev_pos = self.pos_for(&(note.0 + FRAME_TIME as f32, note.1));
            let pos = self.pos_for(note);
            self.canvas.draw_line(prev_pos, pos);
        }
        self.droplets.draw(&mut self.canvas);

        if RECORD {
            self.ffmpeg
                .as_mut()
                .map(|ffmpeg| ffmpeg.write_all(&self.canvas.buffer.as_slice()));
        }
        self.canvas.display();
    }

    fn pos_for(&self, note: &(f32, u8)) -> Vec2 {
        let (time, note) = note;
        let time_left = time - self.time;
        let y = map(time_left, 0f32, VIEW, HEIGHT as f32, 0f32);
        let slope_offset = map(y, 0.0, HEIGHT as f32, 0.0, SLOPE);
        let (low, high) = self.note_lowest_highest;
        let x = map(
            *note as f32,
            low as f32,
            high as f32,
            SLOPE,
            WIDTH as f32 - SLOPE,
        );
        Vec2::new(x + slope_offset, y)
    }

    fn update_visible_notes(&mut self) {
        self.time = self.frame as f32 * FRAME_TIME as f32;
        let skip = NOTES.partition_point(|(note_time, key)| note_time < &self.time);
        self.visible_notes = NOTES
            .iter()
            .skip(skip)
            .take_while(|(note_time, key)| note_time < &(self.time + VIEW))
            .copied()
            .collect();
    }

    fn canvas() -> Canvas {
        let mut palette: Vec<_> = PALETTE.iter().map(hex_to_rgb).collect();
        //palette.extend([[0, 0, 0, 0]].repeat(1));
        Canvas::new(palette)
    }

    fn ffmpeg() -> Option<ChildStdin> {
        let ffmpeg_command = "/usr/bin/ffmpeg";
        let args = "-y -f rawvideo -vcodec rawvideo -s 640x480 -pix_fmt rgba -r 30 -i - -an -vcodec h264 -pix_fmt yuv420p -crf 15 /home/kirinokirino/Media/video.mp4".split(' ');
        if RECORD {
            Some(
                Command::new(ffmpeg_command)
                    .args(args)
                    .stdin(Stdio::piped())
                    .spawn()
                    .expect("failed to execute process")
                    .stdin
                    .take()
                    .unwrap(),
            )
        } else {
            None
        }
    }
}

enum BlendMode {
    Replace,
    Blend,
}

struct Canvas {
    pub buffer: Vec<u8>,
    palette: Vec<[u8; 4]>,
    pub pen_color: [u8; 4],
    blend_mode: BlendMode,
}

impl Canvas {
    pub fn new(palette: Vec<[u8; 4]>) -> Self {
        let mut buffer = Vec::with_capacity(WIDTH * HEIGHT * 4);
        unsafe {
            buffer.set_len(buffer.capacity());
        }
        buffer.fill(255);
        let pen_color = [255, 255, 255, 255];
        Self {
            buffer,
            palette,
            pen_color,
            blend_mode: BlendMode::Replace,
        }
    }

    pub fn select_color(&mut self, color: u8) {
        self.pen_color = self.palette[color as usize % self.palette.len()]
    }

    pub fn dim(&mut self, value: i16) {
        self.buffer.iter_mut().for_each(|v| {
            let new = (*v as i16 + value).min(255).max(0) as u8;
            *v = new;
        });
    }

    pub fn display(&self) {
        let file = std::fs::File::options()
            .create(true)
            .read(true)
            .write(true)
            .open("/tmp/imagesink")
            .unwrap();
        let size = 640 * 480 * 4;
        file.set_len(size.try_into().unwrap()).unwrap();
        let mut mmap = unsafe { memmap2::MmapMut::map_mut(&file).unwrap() };
        if let Some(err) = mmap.lock().err() {
            panic!("{err}");
        }
        let _ = (&mut mmap[..]).write_all(&self.buffer.as_slice());
    }

    fn random(&mut self) {
        for i in 0..self.buffer.len() / 4 {
            let mut change = self.palette[fastrand::usize(0..self.palette.len())];
            change[3] = (change[3] as f32 * 0.05) as u8;
            self.pen_color = change;
            self.point_blend(i * 4);
        }
    }

    fn draw_curve(&mut self, start: Vec2, control: Vec2, end: Vec2) {
        let points = start.distance(control) + control.distance(end) + end.distance(start);
        for i in 1..points as usize {
            let proportion = i as f32 / points;
            let path1 = control - start;
            let point1 = start + path1 * proportion;
            let path2 = end - control;
            let point2 = control + path2 * proportion;
            let path3 = point2 - point1;
            let point3 = point1 + path3 * proportion;
            self.draw_point(point3);
        }
    }

    fn draw_line(&mut self, from: Vec2, to: Vec2) {
        let delta = to - from;
        let axis_biggest_distance = (delta.x).abs().max((delta.y).abs()) as usize;
        let normalized = delta.normalize();
        for step in 0..axis_biggest_distance {
            let magnitude = step as f32;
            let x = from.x + normalized.x * magnitude;
            let y = from.y + normalized.y * magnitude;
            self.draw_point(Vec2::new(x, y));
        }
    }

    fn draw_circle(&mut self, pos: Vec2, radius: f32) {
        let left_x = (pos.x - radius) as usize;
        let right_x = (pos.x + radius) as usize;
        let top_y = (pos.y - radius) as usize;
        let bottom_y = (pos.y + radius) as usize;
        for offset_x in left_x..=right_x {
            for offset_y in top_y..=bottom_y {
                if ((offset_x as f32 - pos.x as f32).powi(2)
                    + (offset_y as f32 - pos.y as f32).powi(2))
                .sqrt()
                    < radius
                {
                    self.draw_point(Vec2::new(offset_x as f32, offset_y as f32));
                }
            }
        }
    }

    fn draw_square(&mut self, top_left: Vec2, bottom_right: Vec2) {
        for offset_x in top_left.x as usize..=bottom_right.x as usize {
            for offset_y in top_left.y as usize..=bottom_right.y as usize {
                self.draw_point(Vec2::new(offset_x as f32, offset_y as f32));
            }
        }
    }

    fn draw_point(&mut self, pos: Vec2) {
        if pos.x >= 640.0 || pos.x < 0.0 || pos.y >= 480.0 || pos.y < 0.0 {
            return;
        }
        let buffer_idx = self.idx(pos.x as usize, pos.y as usize);
        // if (buffer_idx + 3) > self.buffer.len() {
        //     // TODO err?
        //     return;
        // }
        match self.blend_mode {
            BlendMode::Replace => self.point_replace(buffer_idx),
            BlendMode::Blend => self.point_blend(buffer_idx),
        }
    }

    fn point_blend(&mut self, buffer_idx: usize) {
        let [r, g, b, a] = self.pen_color;

        if a == 0 {
            return;
        } else if a == 255 {
            self.point_replace(buffer_idx);
            return;
        }

        let mix = a as f32 / 255.0;
        let [dst_r, dst_g, dst_b, dst_a] = [
            self.buffer[buffer_idx] as f32,
            self.buffer[buffer_idx + 1] as f32,
            self.buffer[buffer_idx + 2] as f32,
            self.buffer[buffer_idx + 3] as f32,
        ];

        self.buffer[buffer_idx] = ((r as f32 * mix) + (dst_r * (1.0 - mix))) as u8;
        self.buffer[buffer_idx + 1] = ((g as f32 * mix) + (dst_g * (1.0 - mix))) as u8;
        self.buffer[buffer_idx + 2] = ((b as f32 * mix) + (dst_b * (1.0 - mix))) as u8;
        self.buffer[buffer_idx + 3] = ((a as f32 * mix) + (dst_a * (1.0 - mix))) as u8;
    }

    fn point_replace(&mut self, buffer_idx: usize) {
        self.buffer[buffer_idx] = self.pen_color[0];
        self.buffer[buffer_idx + 1] = self.pen_color[1];
        self.buffer[buffer_idx + 2] = self.pen_color[2];
        self.buffer[buffer_idx + 3] = self.pen_color[3];
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        (x + y * WIDTH) * 4
    }
}

fn hex_to_rgb(hex: &&str) -> [u8; 4] {
    let hex = hex.trim_matches('#');
    [
        u8::from_str_radix(&hex[0..2], 16).unwrap(),
        u8::from_str_radix(&hex[2..4], 16).unwrap(),
        u8::from_str_radix(&hex[4..6], 16).unwrap(),
        255,
    ]
}

pub fn map(value: f32, start1: f32, stop1: f32, start2: f32, stop2: f32) -> f32 {
    (value - start1) / (stop1 - start1) * (stop2 - start2) + start2
}

pub fn note_find_lowest_highest() -> (u8, u8) {
    let mut lowest = 255u8;
    let mut highest = 0u8;
    for (_, note) in NOTES {
        if note > highest {
            highest = note;
        }
        if note < lowest {
            lowest = note;
        }
    }
    (lowest, highest)
}
