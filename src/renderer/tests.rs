#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::state::TIMESTEP;
    use crate::body::foil::Foil;
    use crate::renderer::Renderer;
    use ultraviolet::Vec2;

    struct DummyContext {
        pub lines: Vec<(Vec2, Vec2)>,
    }

    impl DummyContext {
        fn new() -> Self { Self { lines: Vec::new() } }
    }

    impl DummyContext {
        pub fn draw_line(&mut self, start: Vec2, end: Vec2, _color: [u8; 4]) {
            self.lines.push((start, end));
        }
    }

    #[test]
    fn constant_current_produces_lines() {
        *TIMESTEP.lock() = 0.001;
        let mut r = Renderer::new();
        r.foils.push(Foil {
            id: 1,
            body_ids: vec![],
            current: 1.0,
            accum: 0.0,
            switch_hz: 0.0,
            link_id: None,
            mode: crate::body::foil::LinkMode::Parallel,
        });
        r.selected_foil_ids.push(1);

        for f in 0..25000 {
            r.frame = f;
            r.update_foil_wave_history();
        }

        let mut ctx = DummyContext::new();
        r.draw_foil_square_waves(&mut ctx);
        assert!(!ctx.lines.is_empty(), "No lines drawn for constant current");
    }
}
