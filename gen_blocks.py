import random

blocks = {}
world = []

for i in range(0, 20):
    for j in range(0, 20):
        w = 100
        x0 = -1000 + i * w
        z0 = -1000 + j * w
        y0 = -1
        x1 = x0 + w
        y1 = random.randrange(0, w)
        z1 = z0 + w
        name = f"block_{i}_{j}"
        blocks[name] = {
            "type": "block",
            "corner0": [x0, y0, z0],
            "corner1": [x1, y1, z1],
            "material": "ground",
        }
        world.append(name)

print(f"BLOCKS:\n{blocks}\nWORLD:\n{world}")
