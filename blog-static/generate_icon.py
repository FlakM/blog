import os
import svgwrite
from PIL import ImageFont

def create_favicon_svg(text, font_path, output_svg_path, size=(64, 64), font_size=48):
    dwg = svgwrite.Drawing(output_svg_path, profile='tiny', size=size)

    # Load the font to get the correct font family name
    font = ImageFont.truetype(font_path, font_size)
    font_family = font.getname()[0]

    # Create a group to center the text
    text_group = dwg.g(transform=f"translate({size[0] / 2}, {size[1] / 2})")

    # Create text element with proper alignment
    text_element = dwg.text(
        text,
        insert=('10', '20'),
        text_anchor='middle',
        font_size=font_size,
        font_family=font_family
    )

    # Add the text element to the group
    text_group.add(text_element)

    # Add the group to the drawing
    dwg.add(text_group)

    # Save the SVG file
    dwg.save()

    print(f"Favicon created at {output_svg_path}")

# Example usage
font_dir = os.environ.get('FONT', '/usr/share/fonts')  # Provide a default path if FONT is not set
create_favicon_svg("{}", f"{font_dir}/truetype/NerdFonts/FiraCodeNerdFont-Light.ttf", "favicon.svg")
