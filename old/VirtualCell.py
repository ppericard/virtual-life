#! /usr/bin/python3

import os
import sys
import time
import random

class Environment:
	def __init__(self, height=30, width=50):
		self.height = height
		self.width = width
		self.matrix = [[0 for j in range(self.width)] for i in range(self.height)]
		self.element_list = list()
		self.__populate()
	
	def run_next_step(self):
		for element in element_list:
			free_positions = self.__get_free_positions(element.i, element.j)
			new_position = element.move(free_positions)
			
	
	def display(self, file_handle=sys.stdout, separator=' '):
		for i in range(self.height):
			for j in range(self.width):
				character = ' '
				if self.matrix[i][j]:
					character = self.matrix[i][j].display_character
				file_handle.write("{0}{1}".format(character, separator))
			file_handle.write('\n')
	
	def __populate(self):
		for i in range(self.height):
			for j in range(self.width):
				if random.random() < 0.01:
					element = Cell(i, j)
					self.matrix[i][j] = element
					self.element_list.append(element)
	
	def __get_free_positions(self, i, j):
		pass

class EnvObject:
	def __init__(self, i, j):
		self.i = i
		self.j = j
		self.display_character = ' '
		
	def move(self, step, i, j, height, width):
		pass

class Cell(EnvObject):
	def __init__(self, i, j):
		super().__init__(i, j)
		self.is_living = True
		self.display_character = 0

if __name__ == '__main__':
	random.seed()
	forest = Environment()
	print(forest.__dict__)
	while True:
		os.system('clear')
		forest.display()
		time.sleep(0.04)

	
	
