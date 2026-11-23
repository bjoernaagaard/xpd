"""Orbit a rectangular prism with the Cairo renderer."""

from manim import DEGREES, UP, Create, Prism, Text, ThreeDAxes, ThreeDScene


class CameraOrbit(ThreeDScene):
    def construct(self) -> None:
        self.camera.background_color = "#0B1220"
        axes = ThreeDAxes(
            x_range=[-2, 2, 1],
            y_range=[-2, 2, 1],
            z_range=[-2, 2, 1],
            x_length=5,
            y_length=5,
            z_length=4,
            axis_config={"stroke_opacity": 0.55},
        )
        prism = Prism(
            dimensions=[2.4, 1.4, 1.0],
            fill_color="#7BC6FF",
            fill_opacity=0.7,
            stroke_color="#D2ECFF",
            stroke_width=1.3,
        )
        title = Text("A prism from two viewpoints", font_size=38).to_edge(UP, buff=0.55)
        self.set_camera_orientation(phi=67 * DEGREES, theta=-45 * DEGREES)
        self.add_fixed_in_frame_mobjects(title)
        self.add(axes)
        self.play(Create(prism), run_time=0.9)
        self.wait(0.45)
        self.move_camera(phi=58 * DEGREES, theta=48 * DEGREES, run_time=2.6)
        self.wait(0.95)
