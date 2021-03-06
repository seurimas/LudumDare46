#![enable(implicit_some)]
Container(
    transform: (
        id: "main_menu",
        anchor: Middle,
        stretch: XY( x_margin: 0.0, y_margin: 0.0, keep_aspect_ratio: true),

        // here the z-value is relevant to get shown `in front of' the other UI elements
        z: 2.0,

        width: 1920.0,
        height: 1080.0,
    ),
    background: SolidColor(0.0, 0.0, 0.0, 0.5),
    children: [
        Button(
            transform: (
                id: "play",
                x: 0.0,
                y: 30.0,
                z: 2.0,
                width: 300.0,
                height: 50.0,
                anchor: Middle,
                mouse_reactive: true,
            ),
            button: (
                text: "Play",
                font_size: 36.0,
                normal_image: SolidColor(0.4, 0.4, 0.4, 1.),
                hover_image: SolidColor(0.5, 0.5, 0.5, 1.),
                press_image: SolidColor(0.2, 0.2, 0.2, 1.),
                normal_text_color: (0.2, 0.2, 0.2, 1.0),
                hover_text_color: (0.7, 0.7, 0.7, 1.0),
                press_text_color: (1.0, 1.0, 1.0, 1.0),
            )
        ),

        Button(
            transform: (
                id: "exit",
                x: 0.0,
                y: -90.0,
                z: 2.0,
                width: 300.0,
                height: 50.0,
                anchor: Middle,
                mouse_reactive: true,
            ),
            button: (
                text: "Exit",
                font_size: 36.0,
                normal_image: SolidColor(0.4, 0.4, 0.4, 1.),
                hover_image: SolidColor(0.5, 0.5, 0.5, 1.),
                press_image: SolidColor(0.2, 0.2, 0.2, 1.),
                normal_text_color: (0.2, 0.2, 0.2, 1.0),
                hover_text_color: (0.7, 0.7, 0.7, 1.0),
                press_text_color: (1.0, 1.0, 1.0, 1.0),
            )
        ),
    ]
)
