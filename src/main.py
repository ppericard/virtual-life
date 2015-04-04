#! /usr/bin/python3

import os
import sys
import time
import random
import collections

class Environment:
    def __init__(self, height=40, width=60):
    # def __init__(self, height=10, width=10):
        self.height = height
        self.width = width
        self.agents_dict = collections.OrderedDict()
        self.__populate()
    
    def run_next_step(self):
        new_agents_dict = dict()
        dict_len = len(self.agents_dict)
        for i in range(dict_len):
            (i, j), element = self.agents_dict.popitem(last=False)
            new_position = element.move(self.get_neighbors(i, j), self.height, self.width)
            self.agents_dict[new_position] = element
    
    def display(self, file_handle=sys.stdout, separator=' '):
        for i in range(self.height):
            for j in range(self.width):
                character = ' '
                if (i, j) in self.agents_dict:
                    character = self.agents_dict[(i, j)].display_character
                file_handle.write("{0}{1}".format(character, separator))
            file_handle.write('\n')
        file_handle.write("num_EnvObject="+str(len(self.agents_dict))+"\n")
    
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

    display_character = ' '
    
    def __init__(self, i, j):
        self.i = i
        self.j = j
    
    def move(self, neighbors_dict, height, width):
        delta = ((-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1))
        # print("old_position = ("+str(self.i)+", "+str(self.j)+") "+"dict_length="+str(len(neighbors_dict)))
        # for (i, j), element in neighbors_dict.items():
            # sys.stdout.write(str(i)+" "+str(j)+" ")
            # print(element.__dict__)
        new_position = self.get_random_new_position(delta, height, width)
        # print("new_position = ("+str(new_position[0])+", "+str(new_position[1])+")")
        while (new_position in neighbors_dict):
            new_position = self.get_random_new_position(delta, height, width)
            # print("new_position = ("+str(new_position[0])+", "+str(new_position[1])+")")
        # if (len(neighbors_dict)):
            # print("new_position = ("+str(new_position[0])+", "+str(new_position[1])+")")
        # print("new_position = ("+str(new_position[0])+", "+str(new_position[1])+")")
        (self.i, self.j) = new_position
        return new_position
    
    def get_random_new_position(self, delta, height, width):
        new_position = (-1, -1)
        while (not self.is_position_in_environment_boundaries(new_position, height, width)):
            (offset_i, offset_j) = random.choice(delta)
            new_position = (self.i+offset_i, self.j+offset_j)
            # print("new_position = ("+str(new_position[0])+", "+str(new_position[1])+")")
        return new_position
    
    def is_position_in_environment_boundaries(self, position, height, width):
        # print(str(position[0])+" "+str(position[1])+" "+str(height)+" "+str(width))
        # print(position[0] >= 0 and position[1] >= 0 and position[0] < height and position[1] < width)
        return (position[0] >= 0 and position[1] >= 0 and
                position[0] < height and position[1] < width)


class Cell(EnvObject):
    
    is_living = True
    display_character = '0'
    
    def __init__(self, i, j):
        super().__init__(i, j)


if __name__ == '__main__':
    random.seed(1)
    # random.seed()
    forest = Environment()
    while True:
    # for i in range(11):
        os.system('clear')
        forest.display()
        forest.run_next_step()
        time.sleep(0.1)

    
    
