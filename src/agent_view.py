

class AgentView:

    display_character = '0'

    def __init__(self, agent):
        self.agent = agent


class CellView(AgentView):

    display_character = 'O'

    def __init__(self, cell):
        super().__init__(cell)