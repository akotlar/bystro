"""Query an annotation file and return a list of sample_ids and genes meeting the query criteria."""
import logging
import math
from typing import Any, Callable

from msgspec import Struct
import numpy as np
import pandas as pd
from opensearchpy import OpenSearch

from bystro.utils.config import get_opensearch_config
from bystro.search.utils.opensearch import gather_opensearch_args
from bystro.proteomics.fragpipe_tandem_mass_tag import TandemMassTagDataset

logger = logging.getLogger(__file__)

OPENSEARCH_CONFIG = gather_opensearch_args(get_opensearch_config())
HETEROZYGOTE_DOSAGE = 1
HOMOZYGOTE_DOSAGE = 2
MISSING_GENO_DOSAGE = np.nan
ONE_DAY = "1d"  # default keep_alive time for opensearch point in time index

# The fields to return for each variant matched by the query
OUTPUT_FIELDS = [
    "chrom",
    "pos",
    "ref",
    "alt",
    "refSeq.name2",
    "refSeq.name",
    "refSeq.codonNumber",
    "refSeq.codonPosition",
    "nearestTss.refSeq.name2",
    "nearestTss.refSeq.dist",
    "homozygotes",
    "heterozygotes",
    "missingGenos",
    "clinvarVcf.CLNSIG",
    "cadd",
    "caddIndel",
    "gnomad.genomes.AF",
    "gnomad.exomes.AF",
]


class OpenSearchQueryConfig(Struct):
    """Represent parameters for configuring OpenSearch queries."""

    max_query_size: int = 10_000
    max_slices: int = 1024
    keep_alive: str = ONE_DAY


OPENSEARCH_QUERY_CONFIG = OpenSearchQueryConfig()


def _flatten(xs: Any) -> list[Any]:  # noqa: ANN401 (`Any` is really correct here)
    """Flatten an arbitrarily nested list."""
    if not isinstance(xs, list):
        return [xs]
    return sum([_flatten(x) for x in xs], [])


def _extract_samples(samples):
    return [sample[0] for sample in samples]


# The following fields are fetchhed by allele, and therefore only ever have one position of values
# gnomad, dbSNP, clinvarVcf, cadd, caddIndel
# Among these, gnomad, cadd, caddIndel have only 1 record, so we can just take the [0][0] record (first and only record)
# gnomAD may have multiple overlapping records, so we would need to flatten the [0][0] record
def _get_samples_genes_dosages_from_hit(hit: dict[str, Any]) -> pd.DataFrame:
    """Given a document hit, return a dataframe of samples, genes and dosages."""
    source = hit["_source"]
    chrom = source["chrom"][0][0][0]
    pos = source["pos"][0][0][0]
    ref = source["ref"][0][0][0]
    alt = source["alt"][0][0][0]
    refSeq = source.get("refSeq")

    print("source", source)

    if not refSeq:
        return pd.DataFrame()
    # print('refSeq', refSeq)
    # num_position_records = len(refSeq["name"])
    unique_gene_names = set(_flatten(refSeq["name2"]))
    # name2_is_unique = len(unique_gene_names) == 1

    heterozygotes = source.get("heterozygotes")
    homozygotes = source.get("homozygotes")
    missing_genos = source.get("missingGenos")

    clinvarVcf = source.get("clinvarVcf")
    clinvarVcf_clinsig = None
    if clinvarVcf is not None:
        clinvarVcf_clinsig = clinvarVcf.get("CLNSIG")
        if clinvarVcf_clinsig is not None:
            clinvarVcf_clinsig = _flatten(clinvarVcf_clinsig[0])

    cadd = source.get("cadd")
    if cadd is not None:
        cadd = cadd[0][0][0]
    
    caddIndel = source.get("caddIndel")
    if caddIndel is not None:
        caddIndel = caddIndel['PHRED'][0][0][0]

    gnomAD = source.get("gnomad")
    gnomAD_genomes = None
    gnomAD_exomes = None

    if gnomAD is not None:
        gnomAD_genomes = gnomAD.get("genomes")
        gnomAD_exomes = gnomAD.get("exomes")

    gnomad_genomes_af = None
    if gnomAD_genomes is not None:
        gnomad_genomes_af = gnomAD_genomes.get("AF")

    gnomad_exomes_af = None
    if gnomAD_exomes is not None:
        gnomad_exomes_af = gnomAD_exomes.get("AF")

    if gnomad_genomes_af is not None:
        gnomad_genomes_af = gnomad_genomes_af[0][0][0]

    if gnomad_exomes_af is not None:
        gnomad_exomes_af = gnomad_exomes_af[0][0][0]

    rows = []
    # for 
    for gene_name in unique_gene_names:
        if heterozygotes:
            for heterozygote in _flatten(heterozygotes):
                if heterozygote is None:
                    continue
                rows.append(
                    {
                        "sample_id": str(heterozygote),
                        "chrom": chrom,
                        "pos": pos,
                        "ref": ref,
                        "alt": alt,
                        "gene_name": gene_name,
                        "dosage": HETEROZYGOTE_DOSAGE,
                        "cadd": cadd,
                        "caddIndel": caddIndel,
                        "gnomad_genomes_af": gnomad_genomes_af,
                        "gnomad_exomes_af": gnomad_exomes_af
                    }
                )
        if homozygotes:
            for homozygote in _flatten(homozygotes):
                if homozygote is None:
                    continue
                rows.append(
                    {
                        "sample_id": str(homozygote),
                        "chrom": chrom,
                        "pos": pos,
                        "ref": ref,
                        "alt": alt,
                        "gene_name": gene_name,
                        "dosage": HOMOZYGOTE_DOSAGE,
                        "cadd": cadd,
                        "caddIndel": caddIndel,
                        "gnomad_genomes_af": gnomad_genomes_af,
                        "gnomad_exomes_af": gnomad_exomes_af
                    }
                )
        if missing_genos:
            for missing_geno in _flatten(missing_genos):
                if missing_geno is None:
                    continue
                rows.append(
                    {
                        "sample_id": str(missing_geno),
                        "chrom": chrom,
                        "pos": pos,
                        "ref": ref,
                        "alt": alt,
                        "gene_name": gene_name,
                        "dosage": MISSING_GENO_DOSAGE,
                        "cadd": cadd,
                        "caddIndel": caddIndel,
                        "gnomad_genomes_af": gnomad_genomes_af,
                        "gnomad_exomes_af": gnomad_exomes_af
                    }
                )
    # print("rows", rows)
    return pd.DataFrame(rows)


def _process_response(resp: dict[str, Any]) -> pd.DataFrame:
    """Postprocess query response from opensearch client."""
    samples_genes_dosages_df = pd.concat(
        [_get_samples_genes_dosages_from_hit(hit) for hit in resp["hits"]["hits"]]
    )
    # we may have multiple variants per gene in the results, so we
    # need to drop duplicates here.
    return samples_genes_dosages_df.drop_duplicates()


def _get_num_slices(
    client: OpenSearch,
    index_name: str,
    query: dict[str, Any],
) -> int:
    """Count number of hits for the index."""
    get_num_slices_query = query["body"].copy()
    get_num_slices_query.pop("sort", None)
    get_num_slices_query.pop("track_total_hits", None)

    response = client.count(body=get_num_slices_query, index=index_name)

    n_docs: int = response["count"]
    if n_docs < 1:
        err_msg = (
            f"Expected at least one document in `response['count']`, got response: {response} instead."
        )
        raise RuntimeError(err_msg)

    num_slices_necessary = math.ceil(n_docs / OPENSEARCH_QUERY_CONFIG.max_query_size)
    num_slices_planned = min(num_slices_necessary, OPENSEARCH_QUERY_CONFIG.max_slices)
    return max(num_slices_planned, 1)


def _run_annotation_query(
    query: dict[str, Any],
    index_name: str,
    client: OpenSearch,
) -> pd.DataFrame | None:
    """Given query and index contained in SaveJobData, run query and return results as dataframe."""
    num_slices = _get_num_slices(client, index_name, query)
    point_in_time = client.create_point_in_time(  # type: ignore[attr-defined]
        index=index_name, params={"keep_alive": OPENSEARCH_QUERY_CONFIG.keep_alive}
    )

    query_results = []
    try:  # make sure we clean up the PIT index properly no matter what happens in this block
        pit_id = point_in_time["pit_id"]
        query["body"]["pit"] = {"id": pit_id}
        query["body"]["size"] = OPENSEARCH_QUERY_CONFIG.max_query_size
        for slice_id in range(num_slices):
            slice_query = query.copy()
            if num_slices > 1:
                # Slice queries require max > 1
                slice_query["slice"] = {"id": slice_id, "max": num_slices}
            query_results.append(_process_response(client.search(**query)))
        print("query_results", query_results)
    except Exception as e:
        err_msg = (
            f"Encountered exception: {e!r} while running opensearch_query, "
            "deleting PIT index and exiting.\n"
            f"query: {query}\n"
            f"client: {client}\n"
            f"opensearch_query_config: {OPENSEARCH_QUERY_CONFIG}\n"
        )
        logger.exception(err_msg, exc_info=e)
    finally:
        client.delete_point_in_time(body={"pit_id": pit_id})  # type: ignore[attr-defined]
    

    return pd.concat(query_results)


def _build_opensearch_query_from_query_string(query_string: str) -> dict[str, Any]:
    base_query = {
        "body": {
            "query": {
                "bool": {
                    "filter": {
                        "query_string": {
                            "default_operator": "AND",
                            "query": query_string,
                            "lenient": True,
                            "phrase_slop": 5,
                            "tie_breaker": 0.3,
                        },
                    },
                },
            },
            "sort": "_doc",
        },
        "_source_includes": OUTPUT_FIELDS,
    }
    return base_query


def get_annotation_result_from_query(
    user_query_string: str,
    index_name: str,
    client: OpenSearch,
) -> pd.DataFrame | None:
    """Given a query and index, return a dataframe of variant / sample_id records matching query."""
    query = _build_opensearch_query_from_query_string(user_query_string)
    return _run_annotation_query(query, index_name, client)


def join_annotation_result_to_proteomics_dataset(
    query_result_df: pd.DataFrame,
    tmt_dataset: TandemMassTagDataset,
    get_tracking_id_from_genomic_sample_id: Callable[[str], str] = (lambda x: x),
    get_tracking_id_from_proteomic_sample_id: Callable[[str], str] = (lambda x: x),
) -> pd.DataFrame:
    """
    Args:
      query_result_df: pd.DataFrame containing result from get_annotation_result_from_query
      tmt_dataset: TamdemMassTagDataset
      get_tracking_id_from_proteomic_sample_id: Callable mapping proteomic sample IDs to tracking IDs
      get_tracking_id_from_genomic_sample_id: Callable mapping genomic sample IDs to tracking IDs
    """
    query_result_df = query_result_df.copy()
    proteomics_df = tmt_dataset.get_melted_abundance_df()

    query_result_df.sample_id = query_result_df.sample_id.apply(get_tracking_id_from_genomic_sample_id)
    proteomics_df.sample_id = proteomics_df.sample_id.apply(get_tracking_id_from_proteomic_sample_id)

    joined_df = query_result_df.merge(
        proteomics_df,
        left_on=["sample_id", "gene_name"],
        right_on=["sample_id", "gene_name"],
    )
    return joined_df
