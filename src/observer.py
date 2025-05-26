"""
This module implements the Observer design pattern.

It provides two base classes:
- `Observable`: For objects that want to be observed. They can register observers
  and notify them of changes.
- `Observer`: For objects that want to observe Observables. They must implement
  an `update` method to react to notifications.

This pattern allows for a decoupled design where objects (Observers) can react
to changes in other objects (Observables) without direct dependencies.
"""
from typing import List, Any # For type hinting

class Observable:
    """
    Represents an object that can be observed by one or more Observers.

    When an Observable changes its state or an event occurs, it can notify all
    registered Observers.
    """
    def __init__(self):
        """Initializes the Observable with an empty list of observers."""
        self.__observers: List[Observer] = [] # Private list to store registered observers

    def register_observer(self, observer: 'Observer'):
        """
        Registers an Observer to receive updates from this Observable.

        Args:
            observer: The Observer object to register. It must have an `update` method.
        """
        if observer not in self.__observers:
            self.__observers.append(observer)

    def notify_observers(self, *args: Any, **kwargs: Any):
        """
        Notifies all registered observers of a change or event.

        Each observer's `update` method will be called with the provided
        arguments and keyword arguments.

        Args:
            *args: Positional arguments to pass to each observer's update method.
            **kwargs: Keyword arguments to pass to each observer's update method.
                      It's common to pass a reference to the observable itself.
        """
        # Iterate over a copy of the list in case an observer tries to
        # unregister itself during notification.
        for observer in self.__observers[:]:
            observer.update(self, *args, **kwargs)


class Observer:
    """
    Represents an object that observes an Observable.

    Observers must implement an `update` method, which is called by the
    Observable when a change or event occurs.
    """
    def __init__(self, observable: Observable):
        """
        Initializes the Observer and registers it with an Observable.

        Args:
            observable: The Observable object to observe.
        """
        observable.register_observer(self)

    def update(self, observable: Observable, *args: Any, **kwargs: Any):
        """
        Called by the Observable when it notifies its observers.

        This method should be overridden by concrete Observer subclasses to
        implement custom logic for reacting to updates.

        Args:
            observable: The Observable object that sent the notification.
            *args: Positional arguments passed by the Observable.
            **kwargs: Keyword arguments passed by the Observable.
        """
        # Default implementation prints the received data.
        # Subclasses should override this.
        print(f"Observer received update from {observable}: args={args}, kwargs={kwargs}")
