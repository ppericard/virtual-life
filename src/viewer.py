
class Viewer:

    def __init__(self):



class AgentView:

    display_character = 'x'

    def __init__(self, agent):
        self.agent = agent


class CellView(AgentView):

    display_character = '0'

    def __init__(self, cell):
        super().__init__(cell)


class TileView:
