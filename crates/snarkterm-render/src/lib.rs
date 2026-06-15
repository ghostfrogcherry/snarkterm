pub struct RenderPlan {
    pub terminal_layer: &'static str,
    pub gutter_layer: &'static str,
}

impl Default for RenderPlan {
    fn default() -> Self {
        Self {
            terminal_layer: "terminal grid, selection, cursor, and scrollback",
            gutter_layer: "snark gutter overlay; not stdout, because lawsuits are paperwork",
        }
    }
}
