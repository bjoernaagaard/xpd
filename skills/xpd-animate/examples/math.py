"""Complete the square and reveal both solutions of a quadratic equation."""

from manim import DOWN, UP, FadeIn, MathTex, Scene, Text, TransformMatchingTex, Write


class CompletingTheSquare(Scene):
    def construct(self) -> None:
        self.camera.background_color = "#0B1220"
        title = Text("Completing the square", font_size=42).to_edge(UP, buff=0.65)
        self.add(title)
        equation = MathTex("x^2", "+", "6x", "=", "7", font_size=62)
        self.play(Write(equation), run_time=0.9)
        self.wait(0.45)

        instruction = Text("Add 9 to both sides", font_size=27, color="#FFC775").shift(DOWN * 1.7)
        next_equation = MathTex(
            "x^2",
            "+",
            "6x",
            "+9",
            "=",
            "7",
            "+9",
            font_size=60,
            tex_to_color_map={"+9": "#FFC775"},
        )
        self.play(FadeIn(instruction), TransformMatchingTex(equation, next_equation), run_time=0.9)
        self.wait(0.8)

        square = MathTex(r"(x+3)^2", "=", "16", font_size=64)
        self.play(TransformMatchingTex(next_equation, square), run_time=0.9)
        self.remove(instruction)
        self.wait(0.7)

        roots = MathTex("x = 1", r"\quad\text{or}\quad", "x = -7", font_size=57, color="#75E4BD")
        roots.shift(DOWN * 1.6)
        self.play(Write(roots), run_time=0.9)
        self.wait(1.3)
