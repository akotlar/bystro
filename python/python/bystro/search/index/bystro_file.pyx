cimport cython
import tarfile
import gzip
from libc.stdint cimport uint32_t
import subprocess
import ray
import sys
from opensearchpy.helpers.actions import bulk
from opensearchpy.client import OpenSearch
from ruamel.yaml import YAML

from bystro.beanstalkd.worker import ProgressPublisher, get_progress_reporter


from math import ceil
from bystro.search.utils.annotation import DelimitersConfig
from bystro.search.utils.opensearch import gather_opensearch_args
cdef inline populate_hash_path(dict row_document, list field_path, field_value: list | str | int | float | None):
    cdef dict current_dict = row_document
    cdef uint32_t i = 0
    for key in field_path:
        i += 1

        if key not in current_dict:
            current_dict[key] = {}
        if i == len(field_path):
            current_dict[key] = field_value
        else:
            current_dict = current_dict[key]

cdef inline convert_value(str value):
    try:
        return int(value)
    except ValueError:
        try:
            return float(value)
        except ValueError:
            return value

# cdef class Parser:
#     cdef:
#         str index_name
#         int chunk_size
#         str field_separator
#         str position_delimiter
#         str overlap_delimiter
#         str value_delimiter
#         str empty_field_char
#         list header_paths
#         int id

#     def __cinit__(self, str index_name, object delimiters, list header_paths, int chunk_size=500):
#         self.index_name = index_name
#         self.chunk_size = chunk_size
#         self.field_separator = delimiters.field
#         self.position_delimiter = delimiters.position
#         self.overlap_delimiter = delimiters.overlap
#         self.value_delimiter = delimiters.value
#         self.empty_field_char = delimiters.empty_field
#         self.header_paths = header_paths

#         self.id = 0

#     cdef process_chunk(self, list chunk):
#         cdef:
#             str line
#             list row
#             list position_values
#             list values
#             list overlap_values
#             list rows = []
#         for line in chunk:
#             if not line:
#                 raise StopIteration

#             _source = {}

#             row = line.strip("\n").split(self.field_separator)
#             for i, field in enumerate(row):
#                 if field == self.empty_field_char:
#                     continue

#                 position_values = []
#                 for pos_value in field.split(self.position_delimiter):
#                     if pos_value == self.empty_field_char:
#                         position_values.append(None)
#                         continue

#                     values = []
#                     values_raw = pos_value.split(self.value_delimiter)
#                     for value in values_raw:
#                         if value == self.empty_field_char:
#                             values.append(None)
#                             continue

#                         overlap_values = []
#                         for overlap_value in value.split(self.overlap_delimiter):
#                             if overlap_value == self.empty_field_char:
#                                 overlap_values.append(None)
#                                 continue

#                             overlap_values.append(convert_value(overlap_value))

#                         values.append(overlap_values)

#                     position_values.append(values)

#                 populate_hash_path(_source, self.header_paths[i], position_values)

#             if not _source:
#                 continue

#             self.id += 1
#             rows.append({"_index": self.index_name, "_id": self.id, "_source": _source})

#         return rows

cpdef process_chunk(index_name: str, delimiters: DelimitersConfig, header_fields: list, search_client_args: dict, chunk: list):
    cdef:
        str line
        list row
        list position_values
        list values
        list overlap_values
        list rows = []
        str field_separator = delimiters.field
        str position_delimiter = delimiters.position
        str overlap_delimiter = delimiters.overlap
        str value_delimiter = delimiters.value
        str empty_field_char = delimiters.empty_field
        int has_pos_delimiter = 0
        int has_overlap_delimiter = 0
        int has_value_delimiter = 0
        object _source = {}

    # client = OpenSearch(**search_client_args)
    # print("len(header_fields)", len(header_fields), header_fields)
    for line in chunk:
        if not line:
            raise StopIteration

        _source = {}

        row = line.strip("\n").split(field_separator)
        # print("len row", len(row))
        for i, field in enumerate(row):
            if field == delimiters.empty_field:
                continue
                
            has_pos_delimiter = position_delimiter in field
            has_overlap_delimiter = overlap_delimiter in field
            has_value_delimiter = value_delimiter in field

            if not has_pos_delimiter and not has_overlap_delimiter and not has_value_delimiter:
                _source[header_fields[i]] = convert_value(field)
                # populate_hash_path(_source, header_paths[i], convert_value(field))
                continue

            position_values = []
            for pos_value in field.split(position_delimiter):
                if pos_value == empty_field_char:
                    position_values.append(None)
                    continue

                values = []
                values_raw = pos_value.split(value_delimiter)
                for value in values_raw:
                    if value == empty_field_char:
                        values.append(None)
                        continue

                    overlap_values = []
                    for overlap_value in value.split(overlap_delimiter):
                        if overlap_value == empty_field_char:
                            overlap_values.append(None)
                            continue

                        overlap_values.append(convert_value(overlap_value))

                    values.append(overlap_values)

                position_values.append(values)

            _source[header_fields[i]] = position_values
                
            # populate_hash_path(_source, header_paths[i], position_values)

        if not _source:
            continue

        rows.append({"_index": index_name, "_source": _source})
    # res = bulk(client, rows, chunk_size=1000)
    # print(res)

    # client.close()

    del rows

cdef class ReadAnnotationTarball:
    cdef:
        str index_name
        int chunk_size
        object delimiters
        object decompressed_data
        object client
        object post_index_settings
        list header_fields
        list header_paths
        dict search_client_args
        int id

    def __cinit__(self, str index_name, object delimiters, str annotation_name = 'annotation.tsv.gz', int chunk_size=20000):
        self.index_name = index_name
        self.chunk_size = chunk_size
        self.delimiters = delimiters

        annotation_file_name = annotation_name

        with open("/home/ubuntu/bystro/config/hg19.mapping.yml") as f:
            mapping_conf = YAML(typ="safe").load(f)

        with open("/home/ubuntu/bystro/config/elastic-config2.yml") as f:
            search_conf = YAML(typ="safe").load(f)

        self.search_client_args = gather_opensearch_args(search_conf)
        self.client = OpenSearch(**self.search_client_args)

        self.post_index_settings = mapping_conf["post_index_settings"]

        index_body: dict[str, dict] = {
            "settings": mapping_conf["index_settings"],
            "mappings": mapping_conf["mappings"],
        }

        # if not index_body["settings"].get("number_of_shards"):
        #     file_size = os.path.getsize(tar_path)
        #     index_body["settings"]["number_of_shards"] = ceil(
        #         float(file_size) / float(1e10)
            # )

        try:
            self.client.indices.create(index_name, body=index_body)
        except Exception as e:
            print(e)

        # t = tarfile.open(tar_path)
        # for member in t.getmembers():
        #     if annotation_name in member.name:
        #         annotation_file_name = member.name
        # #         ann_fh = t.extractfile(member)
        # #         self.decompressed_data = gzip.GzipFile(fileobj=ann_fh, mode='rb')
        # command = f"tar -xOf {tar_path} {annotation_file_name} | bgzip --threads 8 -d -c"

        # self.decompressed_data = subprocess.Popen(command, stdout=subprocess.PIPE, shell=True, universal_newlines=True)

        header_line = sys.stdin.readline()#self.decompressed_data.stdout.readline()

        self.header_fields = header_line.rstrip("\n").split(self.delimiters.field)
        self.header_paths = [field.split(".") for field in self.header_fields]

        self.id = 0
    
    def read(self, n_threads: int = 8):
        remote_process_chunk = ray.remote(process_chunk)

        # indexers = [
        #                            for _ in range(n_threads)]  # type: ignore  # noqa: E501

        cdef int chunk_iter = 0
        cdef list lines = []
        cdef list ray_workers = []
        for line in sys.stdin:
            # print(line)
            if not line:
                break
            lines.append(line)
            if len(lines) >= self.chunk_size:
                ray_workers.append(remote_process_chunk.remote(index_name = self.index_name, delimiters = self.delimiters,
                                   header_fields = self.header_fields, search_client_args = self.search_client_args, chunk = lines))

                lines = []
                chunk_iter += 1
                if chunk_iter >= n_threads:
                    chunk_iter = 0
        res = ray.get(ray_workers)
        print("final res", res)

        # self.client.indices.put_settings(index=self.index_name, body=self.post_index_settings)
        # self.client.indices.refresh(index=self.index_name)

        # self.client.close()

    def get_header_fields(self):
        return self.header_fields

cpdef ReadAnnotationTarball read_annotation_tarball(str index_name,  object delimiters, str annotation_name = 'annotation.tsv.gz', int chunk_size=20_000):
    return ReadAnnotationTarball(index_name, delimiters, annotation_name, chunk_size)