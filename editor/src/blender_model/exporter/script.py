bl_info = {
	"name": "Export CrystalSphinx Model",
	"blender": (3, 1, 0),
	"category": "Object",
}

import time
import sys
import os
import argparse
import struct
from enum import IntFlag

import bpy
import bmesh
from mathutils import Vector

ERROR_CATEGORY = "SCRIPT_ERROR"
class OutputMode(IntFlag):
	BYTES = 0b01
	TEXT = 0b10
	ALL = 0b11

# Prints an error message to the console which is formatted to be read by the build process
def error(message):
	print(message, file=sys.stderr)

# Parses arguments from the command line which are relevant to this script
def parse_args():
	argv = sys.argv
	if '--' in sys.argv:
		argv = sys.argv[sys.argv.index('--') + 1:]
	parser = argparse.ArgumentParser()
	parser.add_argument("--mesh_name")
	parser.add_argument("--output_mode")
	return parser.parse_known_args(argv)[0]

# Runs the script export process
def run():
	args = parse_args()
	if args.output_mode is None:
		error("MissingArgument(output_mode)")
		return
	if args.mesh_name is None:
		error("MissingArgument(mesh_name)")
		return

	mode = OutputMode[args.output_mode]
	
	mesh = find_mesh(args.mesh_name)
	if mesh is None:
		return

	# Triangulate the found mesh, but don't save it back to file
	bm = bmesh.new()
	bm.from_mesh(mesh)
	bmesh.ops.triangulate(bm, faces=bm.faces[:])
	mesh = bpy.data.meshes.new("triangulated")
	bm.to_mesh(mesh)
	bm.free()
	
	write(mode, OutputMode.TEXT, 'vertex_count=')
	write(mode, OutputMode.ALL, len(mesh.vertices), '>I')
	write(mode, OutputMode.TEXT, '\n')
	for idx,vertex in enumerate(mesh.vertices):
		write(mode, OutputMode.TEXT, f'{idx:03d}: ')

		write(mode, OutputMode.TEXT, 'pos=<')
		write(mode, OutputMode.ALL, vertex.co[0], '>f', '{:+.4f}')
		write(mode, OutputMode.TEXT, ', ')
		write(mode, OutputMode.ALL, vertex.co[1], '>f', '{:+.4f}')
		write(mode, OutputMode.TEXT, ', ')
		write(mode, OutputMode.ALL, vertex.co[2], '>f', '{:+.4f}')
		write(mode, OutputMode.TEXT, '> ')

		write(mode, OutputMode.TEXT, 'groups:')
		write(mode, OutputMode.ALL, len(vertex.groups), '>I')
		write(mode, OutputMode.TEXT, '=[')
		for vertex_group in vertex.groups:
			write(mode, OutputMode.TEXT, '(')
			write(mode, OutputMode.ALL, vertex_group.group, '>I')
			write(mode, OutputMode.TEXT, ', ')
			write(mode, OutputMode.ALL, vertex_group.weight, '>f', '{:.2f}')
			write(mode, OutputMode.TEXT, '),')
		write(mode, OutputMode.TEXT, ']\n')
	write(mode, OutputMode.TEXT, '\n')

	uv_layer = mesh.uv_layers[0].data
	write(mode, OutputMode.TEXT, 'polygon_count=')
	write(mode, OutputMode.ALL, len(mesh.polygons), '>I')
	write(mode, OutputMode.TEXT, '\n')
	for idx,poly in enumerate(mesh.polygons):
		if len(poly.vertices) != 3:
			# https://blender.stackexchange.com/a/19254
			error("NGonsNotSupported")
			return
		write(mode, OutputMode.TEXT, f'{idx:03d}: ')

		write(mode, OutputMode.TEXT, 'normal=<')
		write(mode, OutputMode.ALL, poly.normal[0], '>f', '{:+.4f}')
		write(mode, OutputMode.TEXT, ', ')
		write(mode, OutputMode.ALL, poly.normal[1], '>f', '{:+.4f}')
		write(mode, OutputMode.TEXT, ', ')
		write(mode, OutputMode.ALL, poly.normal[2], '>f', '{:+.4f}')
		write(mode, OutputMode.TEXT, '> ')
		
		vert_loops = {}
		loop_start = poly.loop_start
		loop_end = poly.loop_start + poly.loop_total
		for loop_idx in range(loop_start, loop_end):
			vertex_index = mesh.loops[loop_idx].vertex_index
			vert_loops[vertex_index] = uv_layer[loop_idx].uv

		write(mode, OutputMode.TEXT, 'vertices:')
		write(mode, OutputMode.ALL, len(poly.vertices), '>I')
		write(mode, OutputMode.TEXT, '=[\n')
		for index in poly.vertices:
			write(mode, OutputMode.TEXT, '\tidx=')
			write(mode, OutputMode.ALL, index, '>I')

			uv = vert_loops[index]
			write(mode, OutputMode.TEXT, ' uv=<')
			write(mode, OutputMode.ALL, uv[0], '>f', '{:+.4f}')
			write(mode, OutputMode.TEXT, ', ')
			write(mode, OutputMode.ALL, uv[1], '>f', '{:+.4f}')
			write(mode, OutputMode.TEXT, '>,\n')
		write(mode, OutputMode.TEXT, ']\n')
	write(mode, OutputMode.TEXT, '\n')

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

def write(mode, desired_modes, primitive, bytes_format=None, text_format=None):
	if mode & desired_modes != mode:
		return
	if mode == OutputMode.TEXT:
		if text_format is None:
			data = str(primitive)
		else:
			data = f'{text_format}'.format(primitive)
		data = bytes(data, 'utf-8')
	else:
		if bytes_format is None:
			data = primitive
		else:
			data = struct.pack(bytes_format, primitive)
	if data is not None:
		sys.stdout.buffer.write(data)

if __name__ == "__main__":
	run()
		