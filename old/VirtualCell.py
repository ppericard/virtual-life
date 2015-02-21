#! /usr/bin/python3

import os
import sys
import time
import random

class Environment:
	def __init__(self, height=30, width=50):
		self.height = height
		self.width = width
		self.agents_dict = dict()
		self.__populate()
	
	def run_next_step(self):
		for (i, j), element in self.agents_dict.items():
			new_position = element.move(self.get_neighbors(i, j))
			self.agents_dict[new_position] = self.agents_dict.pop((i, j))
	
	def display(self, file_handle=sys.stdout, separator=' '):
		for i in range(self.height):
			for j in range(self.width):
				character = ' '
				if (i, j) in self.agents_dict:
					character = self.agents_dict[(i, j)].display_character
				file_handle.write("{0}{1}".format(character, separator))
			file_handle.write('\n')
	
	def __populate(self):
		for i in range(self.height):
			for j in range(self.width):
				if random.random() < 0.01:
					element = Cell(i, j)
					self.agents_dict[(i, j)] = element
	
	def get_neighbors(self, i, j):
		delta = ((-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1))
		neighbors_dict = dict()
		for (offset_i, offset_j) in delta:
			neighbor_position = (i+offset_i, j+offset_j)
			if neighbor_position in self.agents_dict:
				neighbors_dict[neighbor_position] = self.agents_dict[neighbor_position]
		return neighbors_dict


class EnvObject:
	def __init__(self, i, j):
		self.i = i
		self.j = j
		self.display_character = ' '
		
	def move(self, neighbors_dict):
		

class Cell(EnvObject):
	def __init__(self, i, j):
		super().__init__(i, j)
		self.is_living = True
		self.display_character = '0'

if __name__ == '__main__':
	random.seed()
	forest = Environment()
	for _ in range(100):
		os.system('clear')
		forest.display()
		#~ forest.run_next_step()
		time.sleep(0.04)

	
	
