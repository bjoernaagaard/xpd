"""Keep a right angle visible as a triangle changes shape."""

from manim import (
    DOWN,
    RIGHT,
    UP,
    Create,
    Dot,
    Line,
    Polygon,
    RightAngle,
    Scene,
    Text,
    ValueTracker,
    always_redraw,
)


class MovingGeometry(Scene):
    def construct(self) -> None:
        self.camera.background_color = "#0B1220"
        title = Text("A changing right triangle", font_size=40).to_edge(UP, buff=0.58)
        invariant = Text("The angle at the base stays at 90°.", font_size=25, color="#75E4BD")
        invariant.to_edge(DOWN, buff=0.55)
        self.add(title, invariant)

        origin = DOWN * 1.55
        base_end = origin + RIGHT * 3.1
        height = ValueTracker(1.25)
        triangle = always_redraw(
            lambda: Polygon(
                origin,
                base_end,
                origin + UP * height.get_value(),
                color="#7BC6FF",
                fill_color="#7BC6FF",
                fill_opacity=0.15,
            )
        )
        vertex = Dot(origin, color="#75E4BD")
        marker = RightAngle(
            Line(origin, base_end), Line(origin, origin + UP), length=0.26, color="#75E4BD"
        )
        self.play(Create(triangle), run_time=0.65)
        self.add(vertex, marker)
        self.play(height.animate.set_value(3.45), run_time=2)
        self.wait(0.45)
        self.play(height.animate.set_value(2.35), run_time=1.3)
        self.wait(1)
