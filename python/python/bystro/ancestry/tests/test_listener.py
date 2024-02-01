"""Test ancestry listener."""

from unittest.mock import patch

from bystro.ancestry.listener import (
    AncestryJobData,
    handler_fn_factory,
)
from bystro.ancestry.tests.test_inference import (
    ANCESTRY_MODEL,
    FAKE_GENOTYPES,
    FAKE_GENOTYPES_DOSAGE_MATRIX,
)
from bystro.beanstalkd.messages import ProgressMessage
from bystro.beanstalkd.worker import ProgressPublisher


handler_fn = handler_fn_factory(ANCESTRY_MODEL)


def test_handler_fn_happy_path():
    progress_message = ProgressMessage(submissionID="my_submission_id")
    publisher = ProgressPublisher(
        host="127.0.0.1", port=1234, queue="my_queue", message=progress_message
    )
    ancestry_job_data = AncestryJobData(submissionID="my_submission_id2", dosage_matrix_path="some_path")
    with patch("bystro.ancestry.listener._load_vcf", return_value=FAKE_GENOTYPES_DOSAGE_MATRIX) as _mock:
        ancestry_response = handler_fn(publisher, ancestry_job_data)

    # Demonstrate that all expected sample_ids are accounted for
    samples_seen = set()
    expected_samples = set(FAKE_GENOTYPES.columns)
    for result in ancestry_response.results:
        samples_seen.add(result.sample_id)

    assert samples_seen == expected_samples
