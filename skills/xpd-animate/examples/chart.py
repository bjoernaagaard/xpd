"""Reveal monthly sales and highlight the March increase."""

from manim import DOWN, LEFT, UP, Axes, Create, FadeIn, GrowFromEdge, Rectangle, Scene, Text


class SalesChart(Scene):
    def construct(self) -> None:
        self.camera.background_color = "#0B1220"
        title = Text("Monthly sales", font_size=42).to_edge(UP, buff=0.5)
        units = Text("Units sold", font_size=25).next_to(title, DOWN, buff=0.18)
        axes = Axes(
            x_range=[0, 5, 1],
            y_range=[0, 120, 30],
            x_length=10,
            y_length=4.1,
            tips=False,
            x_axis_config={"include_ticks": False},
            y_axis_config={"include_numbers": True, "font_size": 20},
        ).shift(DOWN * 0.62)
        self.add(title, units)
        self.play(Create(axes), run_time=0.65)

        for position, (month, sales) in enumerate(
            (("Jan", 42), ("Feb", 50), ("Mar", 94), ("Apr", 79)), start=1
        ):
            color = "#FFC775" if month == "Mar" else "#7BC6FF"
            base = axes.c2p(position, 0)
            top = axes.c2p(position, sales)
            height = top[1] - base[1]
            bar = Rectangle(
                width=1.0, height=height, fill_color=color, fill_opacity=0.9, stroke_width=0
            ).move_to((base + top) / 2)
            label = Text(month, font_size=24).next_to(base, DOWN, buff=0.15)
            value = Text(str(sales), font_size=22, color=color).next_to(bar, UP, buff=0.09)
            self.play(GrowFromEdge(bar, DOWN), FadeIn(label), run_time=0.47)
            self.play(FadeIn(value), run_time=0.18)

        insight = Text("March: 44 more units than February", color="#FFC775", font_size=23)
        insight.to_edge(DOWN, buff=0.28).to_edge(LEFT, buff=0.8)
        self.play(FadeIn(insight), run_time=0.28)
        self.wait(1.25)
