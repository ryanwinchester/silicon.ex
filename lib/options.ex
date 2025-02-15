#  This Source Code Form is subject to the terms of the Mozilla Public
#  License, v. 2.0. If a copy of the MPL was not distributed with this
#  file, You can obtain one at http://mozilla.org/MPL/2.0/.

defmodule Silicon.Options do
  @moduledoc """
    Option structs have some pre-populated values in order to match the defaults of Silicon the binary.
  """
end

# Should this be a tuple?
defmodule Silicon.RGBA do
  use TypeCheck

  defstruct [:r, :g, :b, a: 255]

  @type! u8 :: 0..255
  @type! t :: %Silicon.RGBA{
           r: u8(),
           g: u8(),
           b: u8(),
           a: u8()
         }
end

defmodule Silicon.Options.Shadow do
  use TypeCheck

  defstruct background: {:solid, %Silicon.RGBA{r: 0xAA, g: 0xAA, b: 0xFF}},
            shadow_color: %Silicon.RGBA{r: 0x55, g: 0x55, b: 0x55},
            blur_radius: 0.0,
            pad_horiz: 80,
            pad_vert: 100,
            offset_x: 0,
            offset_y: 0

  @type! u32 :: 0..0xFFFFFFFF
  @type! i32 :: -0x80000000..0x7FFFFFFF
  # TODO: Support images for the background
  @type! background :: {:solid, Silicon.RGBA.t()}

  @type! t :: %__MODULE__{
           background: background(),
           shadow_color: Silicon.RGBA.t(),
           blur_radius: float() | nil,
           pad_horiz: u32() | nil,
           pad_vert: u32() | nil,
           offset_x: i32() | nil,
           offset_y: i32() | nil
         }
end

defmodule Silicon.Options.Image do
  use TypeCheck

  defstruct [
    :line_number,
    :highlight_lines,
    :window_controls,
    :window_title,
    :round_corner,
    font: [{"Hack", 26.0}],
    shadow_adder: %Silicon.Options.Shadow{},
    tab_width: 8,
    line_pad: 2,
    line_offset: 1
  ]

  @type! u32 :: 0..0xFFFFFFFF
  @type! i32 :: -0x80000000..0x7FFFFFFF
  @type! t :: %__MODULE__{
           # Pad between lines
           line_pad: u32() | nil,
           # Show line number
           line_number: boolean() | nil,
           # Font of english character, should be mono space font
           # Silicon library docs say it will use Hack font (size: 26.0) by default
           font: [{String.t(), number()}] | nil,
           # Highlight lines
           highlight_lines: [u32()] | nil,
           # Whether show the window controls
           window_controls: boolean() | nil,
           # Window title
           window_title: String.t() | nil,
           # Whether round the corner of the image
           round_corner: boolean() | nil,
           # Shadow adder,
           shadow_adder: Silicon.Options.Shadow.t() | nil,
           # Tab width
           tab_width: 0..255 | nil,
           # Line Offset
           line_offset: u32() | nil
         }
end

defmodule Silicon.Options.Format do
  use TypeCheck
  @type! theme_resource :: reference()
  @type! theme :: String.t() | theme_resource()

  defstruct [:lang, :theme_set, :image_options, theme: "Dracula"]

  @type! t :: %__MODULE__{
           lang: String.t(),
           theme: theme(),
           image_options: Silicon.Options.Image.t() | nil
         }
end
