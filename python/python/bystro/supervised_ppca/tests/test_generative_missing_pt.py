from numpy.random import default_rng
from bystro.supervised_ppca.gf_generative_missing_pt import PPCAM
from numpy import nan
import numpy as np
# Set up the random number generator
rng = default_rng(seed=42)


def test_ppcam_fit():
    # Generate synthetic data for testing
    n_samples, n_covariates, n_components = 100, 5, 2
    X1 = rng.normal(size=(n_samples, n_covariates))
    print("X1", X1)
    X = np.array([[0.69417657, 0.94520136,        nan,        nan, 0.54996337],
 [       nan, 0.65113087,        nan,        nan, 0.94220754],
 [0.53281241, 0.27764823,        nan,        nan, 0.11060105],
 [0.03923559, 0.93194713,        nan,        nan, 0.90755409],
 [0.78180641, 0.04699175,        nan,        nan, 0.37519607],
 [0.5993672 , 0.63330739, 0.36730241, 0.4714706 , 0.80666081],
 [       nan, 0.90216793, 0.86541394,        nan, 0.94736237],
 [0.5235887 , 0.54809229,        nan,        nan, 0.379839  ],
 [0.10422175, 0.58225451, 0.55335824, 0.68790808, 0.1095685 ],
 [       nan, 0.59420191, 0.41504236,        nan, 0.8516207 ]])

    ppcam = PPCAM(n_components=n_components)
    ppcam.fit(X)

    # Test if the model is fitted
    assert ppcam.W_ is not None
    assert ppcam.sigma2_ is not None
    assert ppcam.p is not None


def test_ppcam_get_covariance():
    # Generate synthetic data for testing
    n_samples, n_covariates, n_components = 100, 5, 2
    X = rng.normal(size=(n_samples, n_covariates))
    
    ppcam = PPCAM(n_components=n_components)
    ppcam.fit(X)

    # Test get_covariance method
    covariance_matrix = ppcam.get_covariance()
    assert covariance_matrix.shape == (n_covariates, n_covariates)


def test_ppcam_get_noise():
    # Generate synthetic data for testing
    n_samples, n_covariates, n_components = 100, 5, 2
    X = rng.normal(size=(n_samples, n_covariates))

    ppcam = PPCAM(n_components=n_components)
    ppcam.fit(X)

    # Test get_noise method
    noise_matrix = ppcam.get_noise()
    assert noise_matrix.shape == (n_covariates, n_covariates)
