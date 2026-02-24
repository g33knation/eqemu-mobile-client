import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d.art3d import Poly3DCollection
import numpy as np

vertices = []
faces = []

with open('/home/tommy/Desktop/MobileClient/web/head_only.obj', 'r') as f:
    for line in f:
        if line.startswith('v '):
            parts = line.split()
            vertices.append([float(parts[1]), float(parts[2]), float(parts[3])])
        elif line.startswith('f '):
            parts = line.split()
            faces.append([int(parts[1])-1, int(parts[2])-1, int(parts[3])-1])

vertices = np.array(vertices)

fig = plt.figure(figsize=(8, 8))
ax = fig.add_subplot(111, projection='3d')

if len(faces) > 0:
    poly3d = [[vertices[vert_id] for vert_id in face] for face in faces]
    ax.add_collection3d(Poly3DCollection(poly3d, facecolors='cyan', linewidths=0.5, edgecolors='k', alpha=0.9))

# Plot limits
if len(vertices) > 0:
    ax.set_xlim([vertices[:,0].min(), vertices[:,0].max()])
    ax.set_ylim([vertices[:,1].min(), vertices[:,1].max()])
    ax.set_zlim([vertices[:,2].min(), vertices[:,2].max()])

ax.set_xlabel('X')
ax.set_ylabel('Y')
ax.set_zlabel('Z')

plt.savefig('/home/tommy/Desktop/MobileClient/web/head_geometry.png')
print("Saved head_geometry.png")
