"""Trace insertion sort and mark the sorted prefix after each step."""

from manim import (
    DOWN,
    RIGHT,
    UP,
    AnimationGroup,
    FadeIn,
    FadeOut,
    RoundedRectangle,
    Scene,
    Text,
    VGroup,
)
from manim.typing import Point3D


class SortingTrace(Scene):
    def construct(self) -> None:
        self.camera.background_color = "#0B1220"
        title = Text("Insertion sort", font_size=43).to_edge(UP, buff=0.62)
        subtitle = Text("Insert the next item into the sorted prefix.", font_size=26).next_to(
            title, DOWN, buff=0.2
        )
        self.add(title, subtitle)
        values = [5, 2, 4, 1]
        cards = [self.card(value).move_to(self.slot(index)) for index, value in enumerate(values)]
        self.play(FadeIn(VGroup(*cards)), run_time=0.48)
        self.play(cards[0][0].animate.set_stroke("#75E4BD"), run_time=0.25)

        for index in range(1, len(values)):
            value = values[index]
            chosen = cards[index]
            larger_positions = (i for i in range(index) if values[i] > value)
            target = next(larger_positions, index)
            note = Text(
                f"Insert {value} at position {target + 1}", font_size=27, color="#FFC775"
            ).shift(DOWN * 1.8)
            self.play(FadeIn(note), chosen[0].animate.set_stroke("#FFC775"), run_time=0.27)
            if target != index:
                shifts = []
                for position, card in enumerate(cards[target:index], start=target):
                    shifts.append(card.animate.move_to(self.slot(position + 1)))
                self.play(chosen.animate.shift(UP * 0.9), run_time=0.28)
                self.play(
                    AnimationGroup(*shifts),
                    chosen.animate.move_to(self.slot(target) + UP * 0.9),
                    run_time=0.58,
                )
                self.play(chosen.animate.move_to(self.slot(target)), run_time=0.28)
                values.insert(target, values.pop(index))
                cards.insert(target, cards.pop(index))
            self.play(FadeOut(note), chosen[0].animate.set_stroke("#75E4BD"), run_time=0.26)
        result = Text("Sorted: 1, 2, 4, 5", font_size=27, color="#75E4BD").shift(DOWN * 1.8)
        self.play(FadeIn(result), run_time=0.35)
        self.wait(1.25)

    @staticmethod
    def card(value: int) -> VGroup:
        frame = RoundedRectangle(
            width=1.4,
            height=1.05,
            corner_radius=0.15,
            stroke_color="#4B6485",
            fill_color="#142238",
            fill_opacity=1,
        )
        return VGroup(frame, Text(str(value), font_size=42))

    @staticmethod
    def slot(index: int) -> Point3D:
        return RIGHT * ((index - 1.5) * 1.75) - UP * 0.25
