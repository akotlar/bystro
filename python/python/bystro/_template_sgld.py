"""
This implements a base class for any model using stochastic gradient 
descent-based techniques for inference. 

Objects
-------
BaseSGDModel(training_options=None)
    

Methods
-------
None
"""
import abc
from typing import Any
import cloudpickle  # type: ignore


class BaseSGLDModel(abc.ABC):
    """
    The base class of a model relying on stochastic gradient descent for
    inference
    """

    def __init__(self, training_options=None, prior_options=None):
        if training_options is None:
            training_options = {}
        if prior_options is None:
            prior_options = {}
        self.training_options = self._fill_training_options(training_options)
        self.prior_options = self._fill_prior_options(prior_options)

    @abc.abstractmethod
    def fit(self, *args, **kwargs):
        """
        Method for fitting

        Parameters
        ----------
        *args:
           List of arguments

        *kwargs:
           Key word arguments
        """

    def pickle(self, path):
        """
        Method for saving the model

        Parameters
        ----------
        path : str
            The directory to save the model to
        """
        mydict = {"model": self}
        with open(path, "wb") as f:
            cloudpickle.dump(mydict, f)

    @classmethod
    def unpickle(cls, path):
        """
        Method for loading the model

        Parameters
        ----------
        path : str
            The directory to load the model from
        """
        with open(path, "rb") as f:
            myDict = cloudpickle.load(f)
        return myDict["model"]

    def _fill_training_options(self, training_options):
        """
        This fills any relevant parameters for the learning algorithm

        Parameters
        ----------
        training_options : dict

        Returns
        -------
        training_opts : dict
        """
        default_options = {"n_samples": 25000,"batch_size":100}
        training_opts = {**default_options, **training_options}
        return training_opts

    def _fill_prior_options(
        self, prior_options: dict[str, Any]
    ) -> dict[str, Any]:
        """
        This sets the prior options for our inference scheme

        Parameters
        ----------
        prior_options : dict
            The original prior options passed as a dictionary

        Options
        -------
        TBD

        Returns
        -------
        prior_options : dict
            The prior options with defaults filled in

        """
        default_options: dict[str, Any] = {}
        return {**default_options, **prior_options}

    @abc.abstractmethod
    def _store_samples(self, list_of_samples):
        """
        Saves the learned variables

        Parameters
        ----------
        list_of_samples: list
            List of variables to save
        """

    @abc.abstractmethod
    def _test_inputs(self, *args, **kwargs):
        """
        This performs error checking on inputs for fit

        Parameters
        ----------
        """

    @abc.abstractmethod
    def _transform_training_data(self, *args, **kwargs):
        """
        This converts training data to adequate format

        Parameters
        ----------
        """
