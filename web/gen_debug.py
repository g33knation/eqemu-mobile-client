from PIL import Image

width, height = 256, 256
img = Image.new("RGB", (width, height), "white")
pixels = img.load()

num_squares = 16
square_size = width // num_squares

for y in range(height):
    for x in range(width):
        sq_x = x // square_size
        sq_y = y // square_size
        if (sq_x + sq_y) % 2 == 0:
            pixels[x, y] = (255, 0, 0)
        else:
            pixels[x, y] = (0, 0, 255)

img.save("/home/tommy/Desktop/MobileClient/web/assets/Textures/checkerboard.png")
print("Checkerboard generated.")
