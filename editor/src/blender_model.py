bl_info = {
	"name": "Export CrystalSphinx Model",
	"blender": (3, 1, 0),
	"category": "Object",
}

import bpy
import time
import sys
import argparse
from mathutils import Vector

ERROR_CATEGORY = "SCRIPT_ERROR"

# Prints an error message to the console which is formatted to be read by the build process
def error(message):
	print(f"{ERROR_CATEGORY}: {message}")

# Parses arguments from the command line which are relevant to this script
def parse_args():
	argv = sys.argv
	if '--' in sys.argv:
		argv = sys.argv[sys.argv.index('--') + 1:]
	parser = argparse.ArgumentParser()
	parser.add_argument("--mesh_name")
	parser.add_argument("--output_path")
	return parser.parse_known_args(argv)[0]

# Runs the script export process
def run():
	args = parse_args()
	if args.output_path is None:
		error("MissingArgument(\"output_path\")")
		return
	if args.mesh_name is None:
		error("MissingArgument(\"mesh_name\")")
		return
	#print("Outputting model to \"{}\"".format(args.output_path))

	mesh = find_mesh(args.mesh_name)
	if mesh is None:
		return
	
	# Breakdown of TODOs and notes:
	# Each polygon refers to a set of verticies. These vertices are the minimal/paired down
	# version of all of the vertices that will be needed to render the model. They do NOT contain
	# the UV coordinates (so space can be saved in the blend file).
	# The polygon also refers to the set of loops, which is how vertices can be mapped to uv coordinates.
	# For a default humanoid, the specs look like:
	# 6 cubes, 36 rectangles, 72 tris, 48 non-uv vertices, 144 unique uv'd vertices, 216 vertex->uv loops
	# 
	# The next step will be to export all non-uv vertices, the polygons (we only support triangles), and vertex->uv loops.
	# Rust will then process that data, removing duplicate uv loops, and then expanding the vertices
	# to be uv-inclusive (which means duplicate vertex positions with different uvs). 
	# 
	# Eventually we will also have to export the bone data

	# This is how binary data will be written such that rust can get access to it (instead of going through an intermediates directory/disk).
	sys.stdout.buffer.write(b"some binary data\n")

	uv_layer = mesh.uv_layers[0].data
	for poly in mesh.polygons:
		if len(poly.vertices) != 3:
			# https://blender.stackexchange.com/a/19254
			error("NGonsNotSupported")
			return
		#print("Triangle")
		#print(f"\tNormal={poly.normal}")
		#print(f"\tIndices={list(poly.vertices)}")
		loops = range(poly.loop_start, poly.loop_start + poly.loop_total)
		for loop_idx in loops:
			vertex_idx = mesh.loops[loop_idx].vertex_index
			vertex_pos = mesh.vertices[vertex_idx].co
			uv_coord = uv_layer[loop_idx].uv
			#print(f"\t\t{vertex_idx}: P={vertex_pos} UV={uv_coord}")

# NOTES:
# The association of vertices to bones happens with vertex groups.
# If an armature is found, then the names of the bones will match the names of vertex groups (Object::vertex_groups)
# https://docs.blender.org/api/current/bpy.types.MeshVertex.html#bpy.types.MeshVertex.groups
# https://docs.blender.org/api/current/bpy.types.Object.html#bpy.types.Object.vertex_groups

def find_mesh(mesh_object_name):
	mesh = None

	for obj in bpy.context.scene.objects:
		if obj.name == mesh_object_name:
			if obj.type != 'MESH':
				error(f"MeshHasWrongType(\"{obj.name}\", {obj.type})")
			mesh = obj

	if mesh is None:
		error(f"MeshDoesNotExist(\"{mesh_object_name}\")")
		return None
	
	if mesh.parent is not None:
		if mesh.parent.type != 'ARMATURE':
			error(f"MeshParentIsNotNoneOrArmature(\"{mesh.name}\", \"{mesh.parent.name}\", {mesh.parent.type})")
			return None
	
	if not mesh.data.name in bpy.data.meshes:
		# mesh has been found, but for some reason the mesh data is missing (this indicates a corrupt blender file)
		error(f"MeshDataMissing(\"{mesh.name}\", \"{mesh.data.name}\"")
		return None
	
	return bpy.data.meshes[mesh.data.name]

if __name__ == "__main__":
	run()
		