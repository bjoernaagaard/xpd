"""Show a cache miss, database lookup, cache fill, and subsequent hit."""

from manim import (
    DOWN,
    LEFT,
    RIGHT,
    UP,
    Arrow,
    DashedLine,
    FadeIn,
    FadeOut,
    GrowArrow,
    RoundedRectangle,
    Scene,
    Text,
    VGroup,
)

CACHE_MISS_HOLD_SECONDS = 1.0
BACKGROUND = "#0B1220"
PANEL = "#142238"
BORDER = "#2F4664"
FOREGROUND = "#F1F5FB"
MUTED = "#BACBE0"
REQUEST = "#7BC6FF"
MISS = "#FFC775"
RETURN = "#75E4BD"
FONT = "Noto Sans"


class CacheMiss(Scene):
    def construct(self) -> None:
        self.camera.background_color = BACKGROUND
        title = Text("What happens on a cache miss?", font=FONT, font_size=40, color=FOREGROUND)
        title.to_edge(LEFT, buff=0.8).set_y(3.28)
        self.add(title)

        participants = ((-4.5, "Client"), (0.0, "Cache"), (4.5, "Database"))
        actors = VGroup(
            *(self.actor(name).move_to(RIGHT * x + UP * 2.17) for x, name in participants)
        )
        lifelines = VGroup()
        for x, _ in participants:
            lifeline = DashedLine(
                RIGHT * x + UP * 1.76,
                RIGHT * x + DOWN * 2.83,
                dash_length=0.08,
                color=BORDER,
                stroke_width=2,
            )
            lifelines.add(lifeline)
        self.add(lifelines, actors)

        status_box = RoundedRectangle(
            width=12.6,
            height=0.63,
            corner_radius=0.15,
            stroke_color=BORDER,
            stroke_width=1.5,
            fill_color=PANEL,
            fill_opacity=1,
        ).set_y(-3.28)
        self.add(status_box)
        status = self.status("The client asks for the user record.")
        self.play(FadeIn(status), run_time=0.35)

        self.message(-4.5, 0.0, 1.22, "GET /users/42", REQUEST)
        self.wait(0.45)

        status = self.change_status(status, "Cache miss. The cache asks the database.")
        self.message(0.0, 4.5, 0.52, "SELECT user 42", MISS)
        self.wait(CACHE_MISS_HOLD_SECONDS)

        status = self.change_status(status, "The database returns the record. The cache saves it.")
        self.message(4.5, 0.0, -0.18, "User record", RETURN)
        self.message(0.0, -4.5, -0.88, "200 OK", RETURN)
        self.wait(0.6)

        status = self.change_status(status, "The client asks again. The record is already cached.")
        self.message(-4.5, 0.0, -1.58, "GET /users/42", REQUEST)
        self.message(0.0, -4.5, -2.28, "200 OK · cache hit", RETURN)
        self.change_status(status, "The second request skips the database.")
        self.wait(1.4)

    @staticmethod
    def actor(name: str) -> VGroup:
        card = RoundedRectangle(
            width=3.12,
            height=0.65,
            corner_radius=0.16,
            stroke_color=BORDER,
            stroke_width=1.5,
            fill_color=PANEL,
            fill_opacity=1,
        )
        label = Text(name, font=FONT, color=FOREGROUND, font_size=26)
        return VGroup(card, label)

    @staticmethod
    def status(text: str) -> Text:
        return Text(text, font=FONT, color=MUTED, font_size=23).set_y(-3.28)

    def change_status(self, current: Text, text: str) -> Text:
        replacement = self.status(text)
        self.play(FadeOut(current), FadeIn(replacement), run_time=0.26)
        return replacement

    def message(self, start: float, end: float, y: float, text: str, color: str) -> None:
        arrow = Arrow(
            RIGHT * start + UP * y,
            RIGHT * end + UP * y,
            buff=0.09,
            color=color,
            stroke_width=4,
            tip_length=0.18,
        )
        label = Text(text, font=FONT, font_size=22, color=color).next_to(arrow, UP, buff=0.09)
        self.play(GrowArrow(arrow), FadeIn(label), run_time=0.6)
