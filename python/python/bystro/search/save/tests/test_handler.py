import os
import tempfile
from unittest.mock import patch

import numpy as np
import pandas as pd
import pytest
import ray

from bystro.beanstalkd.worker import get_progress_reporter
from bystro.search.save.handler import _process_query  # Make sure to import your function correctly


@pytest.fixture
def dosage_matrix_path():
    # Temporary directory for the test files
    with tempfile.TemporaryDirectory() as temp_dir:
        # Define the path for the dosage matrix file
        dosage_matrix_file_path = os.path.join(temp_dir, "dosage_matrix.feather")

        # 10 irrelevant loci, and 2 that are in the search response
        dosage_df = pd.DataFrame(
            {
                "locus": [f"locus_{i}" for i in range(10)] + ["chr1:100:A:T", "chr2:200:G:C"],
                "sample1": [0 for _ in range(10)] + [0, 2],
                "sample2": [1 for _ in range(10)] + [0, 2],
                "sample3": [2 for _ in range(10)] + [2, 1],
            }
        )

        dosage_df.to_feather(dosage_matrix_file_path)

        yield dosage_matrix_file_path


@pytest.fixture
def search_client_args():
    return {"host": "localhost", "port": 9200}


@pytest.fixture
def query_args():
    return {"query": {"match_all": {}}, "size": 10}


@pytest.fixture
def mocked_opensearch_response():
    return {
        "hits": {
            "total": {"value": 2},
            "hits": [
                {
                    "_id": 0,
                    "fields": {"chrom": ["chr1"], "pos": ["100"], "inputRef": ["A"], "alt": ["T"]},
                },
                {
                    "_id": 1,
                    "fields": {"chrom": ["chr2"], "pos": ["200"], "inputRef": ["G"], "alt": ["C"]},
                },
            ],
        }
    }


@patch("bystro.search.save.handler.OpenSearch")
def test_process_query(
    OpenSearchMock, search_client_args, query_args, mocked_opensearch_response, dosage_matrix_path
):

    instance = OpenSearchMock.return_value
    instance.search.return_value = mocked_opensearch_response

    reporter = get_progress_reporter()

    with tempfile.NamedTemporaryFile(delete=False) as temp_output_file:
        dosage_chunk_path = temp_output_file.name

    result = ray.get(
        _process_query.remote(
            [query_args],
            search_client_args,
            reporter,
            dosage_matrix_path,
            dosage_chunk_path,
        )
    )

    assert result is not None

    assert np.array_equal(result, np.array([0, 1]))  # 2 document ids

    dosage_df = pd.read_feather(dosage_chunk_path)

    assert set(dosage_df["locus"].values) == set(["chr1:100:A:T", "chr2:200:G:C"])
    assert dosage_df.shape == (2, 4)

    dosage_df = dosage_df.set_index("locus", drop=True)
    assert dosage_df.loc["chr1:100:A:T", "sample1"] == 0
    assert dosage_df.loc["chr1:100:A:T", "sample2"] == 0
    assert dosage_df.loc["chr1:100:A:T", "sample3"] == 2
    assert dosage_df.loc["chr2:200:G:C", "sample1"] == 2
    assert dosage_df.loc["chr2:200:G:C", "sample2"] == 2
    assert dosage_df.loc["chr2:200:G:C", "sample3"] == 1

    os.remove(dosage_chunk_path)
