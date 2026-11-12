"""Check simplint against the most downloaded PyPI projects, so a rule or fix that breaks real code shows up."""

from __future__ import annotations

import argparse
import ast
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import warnings
from collections import Counter
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from itertools import repeat
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Final

from tasks.download import Pinned, download
from tasks.interpreters import Interpreter, minor_version, resolve


@dataclass(frozen=True, slots=True, kw_only=True)
class Package:
    name: str
    version: str
    sha256: str

    @property
    def url(self) -> str:
        # PyPI's stable address for the archive, which redirects to where the file is stored.
        return f"https://files.pythonhosted.org/packages/source/{self.name[0]}/{self.name}/{self.filename}"

    @property
    def filename(self) -> str:
        # The archive name PyPI gives the source distribution.
        return f"{self.name.replace('-', '_')}-{self.version}.tar.gz"


#: The 100 most downloaded projects of https://hugovk.github.io/top-pypi-packages/ on 2026-11-01, pinned so a run is
#: reproducible.
_TOP: Final[tuple[Package, ...]] = (
    Package(
        name="boto3", version="1.43.124", sha256="7f330e53e274a0b8e5982ed3dd4cec9e7e932cf48306c08db8b9e880d09c5c63"
    ),
    Package(
        name="packaging", version="26.3", sha256="94edc256424af38762eb31306eed28beb9f0efc50a8837492c9d6fd6004aed79"
    ),
    Package(
        name="typing-extensions",
        version="4.16.0",
        sha256="dc983d19a509c94dba722ee6abd33940f7c05a89e243c47e907eb4db6f1a43e5",
    ),
    Package(
        name="certifi", version="2026.10.23", sha256="a9895e6e2758b371e0545e321a3a8fd1fcfd9829ada8824a936dfb16aaaf36cd"
    ),
    Package(name="idna", version="3.20", sha256="a7db850025b95ded1eae8a46181a1a6c56c92c96f0e2b005d9ff8dc0210cab44"),
    Package(name="urllib3", version="2.8.0", sha256="63bf2ead4c879426ebf22ef2a781eeb4aa3b4ae798a0435506f8687fd5bb9b63"),
    Package(
        name="charset-normalizer",
        version="3.5.2",
        sha256="39de2a259fc954455c57274dc94c79d5842774e1247a016aff30bc0efed0f4ef",
    ),
    Package(
        name="requests", version="2.34.2", sha256="f288924cae4e29463698d6d60bc6a4da69c89185ad1e0bcc4104f584e960b9ed"
    ),
    Package(
        name="cryptography", version="50.0.2", sha256="7b46165bb56eb4704e2eaaf86f3c940d19154535d9b0ca7d6d590b04060e00d5"
    ),
    Package(name="cffi", version="2.1.1", sha256="dd31f52ea1086513bb9df30f8fcee9b8918323ae067a3d5b78bc826a000712be"),
    Package(name="pluggy", version="1.7.0", sha256="d1eaa46ebb595891b860ab086b4d09c8588af65ebd4361b8e8f4bb8920b90ba8"),
    Package(name="pyyaml", version="6.0.3", sha256="d76623373421df22fb4cf8817020cbb7ef15c725b9d5e45f17e189bfc384190f"),
    Package(
        name="pycparser", version="3.12", sha256="f03387877eab82ce01bbdfd0ba7252ee55a90b1c708d5129e2ce915475cf1db6"
    ),
    Package(
        name="pygments", version="2.21.0", sha256="610ca751c9bc2492b38eb9a38a7fbc93edbbb2d7182edaf34e66ae493dee5c8c"
    ),
    Package(
        name="python-dateutil",
        version="4.0.0",
        sha256="d7295ff14fdc0f92f662100bfb6c551d160ca090f2939f1ec5b9794265c03f06",
    ),
    Package(name="six", version="1.17.0", sha256="ff70335d468e7eb6ec65b95b99d3a2836546063f63acc5171de367e834932a81"),
    Package(
        name="setuptools", version="84.2.1", sha256="3ff8b3b5492a0aa66960f4419365a86b84e013070d4313e4d4a6894f5d64d6bc"
    ),
    Package(
        name="pydantic", version="2.14.3", sha256="1054f237f4d4733e5a30e1e2333bd33a2004ab9ab1510bd053a9a31bb48e232d"
    ),
    Package(
        name="pydantic-core",
        version="2.50.4",
        sha256="aead3b74b68396bf0feab5d57e050b730ded7df55853c1e048a87e00aaf033c6",
    ),
    Package(name="click", version="8.5.0", sha256="ba0d2089de75ea0310e2dde03160e6ca10009947fb95a182f9b54021bb272e34"),
    Package(name="numpy", version="2.5.4", sha256="9a94cf751c9ad8ebaa835bcd3d40dacf8534ad086b88c38029b65123c7999d2a"),
    Package(
        name="botocore", version="1.43.124", sha256="6df80235995e2926e0768e7ad270e774adcd6ff0307da7ab87ffb8c384edb578"
    ),
    Package(name="anyio", version="4.16.0", sha256="59bb03251907983d1603d27c3087ddb1e93299ea2163adcdba37899a4fb12ffb"),
    Package(
        name="annotated-types",
        version="0.8.0",
        sha256="13b2beaad985e05e2d6407ee4c4f35590b11f8d693a258a561055cac8f64cab7",
    ),
    Package(name="pytest", version="9.2.0", sha256="97c4f3b1f3c7414aa36465e09b8220f1d03627993bd51b1282e193753c80557b"),
    Package(
        name="iniconfig", version="2.3.1", sha256="67f4b9c50da0dedf52af349e7749a80a9057a5031199791b906c3bb3ae878960"
    ),
    Package(
        name="typing-inspection",
        version="0.4.5",
        sha256="7f16c411e7930dc79596d42ab44672a3fe5f60eb34b781a1647e7872363120c7",
    ),
    Package(name="h11", version="0.16.0", sha256="4e35b956cf45792e4caa5885e69fba00bdbc6ffafbfa020300e549b208ee5ff1"),
    Package(name="attrs", version="26.1.0", sha256="d03ceb89cb322a8fd706d4fb91940737b6642aa36998fe130a9bc96c985eff32"),
    Package(
        name="markupsafe", version="3.0.4", sha256="2e9ad7dd851bf45fab9f75cbff4cb493fee9979e8d8c7c9c3ee119022518edd6"
    ),
    Package(name="httpx", version="0.28.1", sha256="75e98c5f16b0f35b567856f597f06ff2270a374470a5c2392242528e3e3e42fc"),
    Package(
        name="httpcore", version="1.0.9", sha256="6e34463af53fd2ab5d807f399a9b45ea31c3dfa2276f15a2c3f00afff6e176e8"
    ),
    Package(
        name="s3transfer", version="0.19.3", sha256="8326acc3d056d7f7b8a37110c7d2aea41f67e83f6ae79cd3b5fbcca34c726406"
    ),
    Package(
        name="protobuf", version="7.36.2", sha256="497d0463ff3316681da6c0b9e8d06cb465d61abce00b613ab42226175644d1bb"
    ),
    Package(
        name="python-dotenv", version="1.2.4", sha256="f0d53e69935a851c0dcc78f3ab7aaccd8cabef0b92382b576b824212902873c0"
    ),
    Package(name="pyjwt", version="2.15.1", sha256="4f259e80cdfb6b3fc18a7de51fd1ef9ec79652f25019bae68975ca2468a34df8"),
    Package(name="jinja2", version="3.1.6", sha256="0137fb05990d35f1275a587e9aee6d56da821fc83491a0fb838183be43f66d6d"),
    Package(name="pandas", version="3.1.1", sha256="05fed3e3b7c99b226f160e29d7d6cb7ef39638c758cbfe42b3f24ba34f04b859"),
    Package(
        name="aiobotocore", version="3.10.0", sha256="35480ef11c51e23087273b85d97f80da2f356a46ce5f3fb88e24483cd343ffdb"
    ),
    Package(
        name="starlette", version="1.10.0", sha256="b8399098d4c1e3522734f7235dc79b1fc967f96cf139c81e0fa30479e0685f9b"
    ),
    Package(
        name="opentelemetry-api",
        version="1.45.2",
        sha256="d7b0008e21e5fd61cbe7a7f34ca3b6905a78ab677d9bc395e299359c75cda6b4",
    ),
    Package(
        name="platformdirs", version="4.14.5", sha256="32e9921db7385aa8169bee4fe9501923e5715ba9fb7ea3b9c69b0d8e0639f7a1"
    ),
    Package(name="pillow", version="13.0.0", sha256="cdcd1ba258178f7356d74170c9a81ba4eab6a5d55748bc402a5a2a6fe6be3510"),
    Package(
        name="uvicorn", version="0.54.0", sha256="a2e33cbfaa0306f8e6b0c13e0cb89d7d7a2da3e62b90c66e18c33d9807b28620"
    ),
    Package(
        name="pathspec", version="1.1.1", sha256="17db5ecd524104a120e173814c90367a96a98d07c45b2e10c2f3919fff91bf5a"
    ),
    Package(
        name="rpds-py", version="2026.9.1", sha256="4793ef7f78268b124b73fa933440f01d258bbae01de9fa53e9080c9ab0425a12"
    ),
    Package(
        name="fsspec", version="2026.9.0", sha256="0f08147951c8cb31d844c3547d631053b127863b60be04cf06e121333ee0e2fe"
    ),
    Package(
        name="jsonschema", version="4.26.0", sha256="0c26707e2efad8aa1bfc5b7ce170f3fccc2e4918ff85989ba9ffa9facb2be326"
    ),
    Package(
        name="filelock", version="4.2.9", sha256="96530b7c538d4ea08493ab067247aab7948c54d8cd622cce711c97cccc5d9896"
    ),
    Package(
        name="jmespath", version="1.1.0", sha256="472c87d80f36026ae83c6ddd0f1d05d4e510134ed462851fd5f754c8c3cbb88d"
    ),
    Package(name="tzdata", version="2026.6", sha256="85d2cd9c09f81f0bb29db27759fc83af4f1e620efb814f8c5bf8b4cc4feece6d"),
    Package(
        name="referencing", version="0.37.0", sha256="44aefc3142c5b842538163acb373e24cce6632bd54bdb01b21ad5863489f50d8"
    ),
    Package(
        name="tomlkit", version="0.15.2", sha256="83e32755ba1d20d9340bd069ee97e9f6232ba2346b63ffcba425de01f7e327a6"
    ),
    Package(
        name="jsonschema-specifications",
        version="2025.9.1",
        sha256="b540987f239e745613c7a9176f3edb72b832a4ac465cf02712288397832b5e8d",
    ),
    Package(
        name="multidict", version="7.2.1", sha256="e8af3e6f1bdbc31fd762b094133b478b89988eaebc63cbe82b1d9e3bbd15cede"
    ),
    Package(
        name="markdown-it-py",
        version="4.2.0",
        sha256="04a21681d6fbb623de53f6f364d352309d4094dd4194040a10fd51833e418d49",
    ),
    Package(name="rich", version="15.0.0", sha256="edd07a4824c6b40189fb7ac9bc4c52536e9780fbbfbddf6f1e2502c31b068c36"),
    Package(
        name="aiohttp", version="3.14.5", sha256="5558a7f5a05af9ecf744af91e5baefc436f93c9333e656c27ec253f9a6bbe178"
    ),
    Package(name="yarl", version="1.25.1", sha256="03dd38de09bc213e9a8b29761eec33ee1d5318dac0e49d8af36e4d27830e23a7"),
    Package(name="mdurl", version="0.1.2", sha256="bb413d29f5eea38f31dd4754dd7377d4465116fb207585f97bf925588687c1ba"),
    Package(
        name="propcache", version="0.5.4", sha256="ff6b113f50bc066a698db5d944d2c6dc7507168dd3341e255a8892fd0715a558"
    ),
    Package(name="pip", version="26.3", sha256="6c36ae7da8695be67e3cd9f7d87919667ced53f57ce2ede75d89b5912ed2bc4c"),
    Package(
        name="annotated-doc", version="0.0.5", sha256="c7e58ce09192557605d8bbd92836d7e1d520ac9580096042c0bfd197efacf1bb"
    ),
    Package(
        name="websockets", version="17.4", sha256="09d3939b70df0c8ad2da145ebd50978bccaee1d565e838b0e7921a34d38571eb"
    ),
    Package(
        name="fastapi", version="0.162.0", sha256="6a22c1e2e6e32e0633ee7e170f00009a29b8228b16feb219ee42f32fd8d88a8f"
    ),
    Package(
        name="googleapis-common-protos",
        version="1.76.0",
        sha256="71db0e44a1d4c2dd024ebad44ca67129cc1e2892e4861df466764749d5ef8642",
    ),
    Package(name="pytz", version="2026.6", sha256="3b0fc2727707663d6e517936f444fd8a8b240f1fc32f4bf7ee03274245945284"),
    Package(
        name="frozenlist", version="1.8.0", sha256="3ede829ed8d842f6cd48fc7081d7a41001a56f1f38603f9d49bf3020d59a31ad"
    ),
    Package(name="tqdm", version="4.70.1", sha256="cefd0eca11b2a37a3aee776544d4f4ae913f02688135b5556b8788dfa474afc4"),
    Package(name="pyasn1", version="0.6.5", sha256="6b11616a6bcf3a40e635f082bb232fa57b5e0dd7f517bd7ddd69ec3794508a47"),
    Package(
        name="aiosignal", version="1.4.0", sha256="f47eecd9468083c2029cc99945502cb7708b082c232f9aca65da147157b251c7"
    ),
    Package(
        name="aiohappyeyeballs",
        version="2.7.1",
        sha256="065665c041c42a5938ed220bdcd7230f22527fbec085e1853d2402c8a3615d9d",
    ),
    Package(name="sniffio", version="1.3.1", sha256="f4324edc670a0f49750a81b895f35c3adb843cca46f0530f79fc1babb23789dc"),
    Package(
        name="pydantic-settings",
        version="2.16.2",
        sha256="3dc08ffabed2bbd5dc2dd55ead7b78b4e4c5cd776aecd04310db5f1224caaf46",
    ),
    Package(name="wrapt", version="2.5.2", sha256="551cb49e88b3722faad780303833bc4fa70685fb1773270fb98b40d39d9fddbf"),
    Package(
        name="python-multipart",
        version="0.0.33",
        sha256="b8112424a6abd162ec08d390565dd113ad699f2fe6fef3c987b8bbcbfe9e390d",
    ),
    Package(
        name="google-auth", version="2.62.1", sha256="3c8e1ce2553784ae17fd220e6f88df3281d86ab98019278f436d06dffab17a9e"
    ),
    Package(
        name="pyarrow", version="26.0.1", sha256="93affcbd8a343e98dc19e9ec8fe17271ebf8fe98102d8b7454175074409f8b72"
    ),
    Package(
        name="trove-classifiers",
        version="2026.9.21.13",
        sha256="0a9ebc8d4e2f3e8a22848c5258033035bec17a3012ac3fea16dbaa764489eb71",
    ),
    Package(name="zipp", version="4.1.2", sha256="cf474b90b07ea932637e17c6b9479de2b2882900b0d311bdaf4f5c000a15a97f"),
    Package(name="lxml", version="6.1.3", sha256="45222d94ddd511536f3b2f7d9deae3b2339b4ce0f075f1ca25703b07cad9dd21"),
    Package(
        name="tenacity", version="9.2.1", sha256="a606b5c808d0cded4a359d5b9932d867ff2a6a6b64d37350260fd01bbdf83839"
    ),
    Package(
        name="importlib-metadata",
        version="10.1.0",
        sha256="82b04047fa09792c1a8cfeddfe08bec79e4b13c29c8469b0d3949860f61017e8",
    ),
    Package(
        name="pyasn1-modules",
        version="0.4.2",
        sha256="677091de870a80aae844b1ca6134f54652fa2c8c5a52aa396440ac3106e941e6",
    ),
    Package(name="ruff", version="0.17.4", sha256="f32d178b50e826e00f6c6c1be3f56333068acb9b67fe167143527fa9b81e890d"),
    Package(
        name="sqlalchemy", version="2.1.8", sha256="8bc4081dd7c17c073f2384ad0f5ee968e63b43507b2e024a3523c000a06d70da"
    ),
    Package(name="jiter", version="0.18.0", sha256="a0ca63fd9450a212e4e658d4cfe0ab4977c4f9b0003c68828b843c65e27662ff"),
    Package(
        name="greenlet", version="3.5.6", sha256="8e67c43bdfc88d5fee6db0d3e40175b362fc95fb85f0412d233b9b203c53a575"
    ),
    Package(
        name="hatchling", version="1.33.0", sha256="4c422bd4d554fcff5de5b73e9596f85e41e9f97f7e553d8f7595ecb31a627bef"
    ),
    Package(name="wheel", version="0.48.0", sha256="94800765601e9171bf5d58d066e640662842bcedcbab982b2c90787a2c987322"),
    Package(
        name="pyparsing", version="3.3.3", sha256="928ae7e20211f3b6f3915a72f06a0cfd29ab9d24279dd6346b6b1a7146397d36"
    ),
    Package(name="grpcio", version="1.85.1", sha256="5f530287d5de5f7f66089ead81de44f6787162944dab3922efa65dfdff108a1d"),
    Package(
        name="regex", version="2026.9.29", sha256="8b5fcc4771732191b2b7d1dd68d8f0353f47f8d90b6150f6dce58bf1112442cb"
    ),
    Package(
        name="truststore", version="0.10.5", sha256="30d36967ccaded5cbb38d602c433f53600036c79d502f4533a49b60a03bbefcd"
    ),
    Package(name="scipy", version="1.18.1", sha256="52c4b7422442aba924d03ad4019852b08a92e64ea187b933135687bfe2747307"),
    Package(
        name="grpcio-status",
        version="1.85.1",
        sha256="3f6ba6090d794b7148b23f03355addd56bee2b78174a12784c4d9b4e28c8928e",
    ),
    Package(name="openai", version="3.46.0", sha256="fb34cd8cab6809ad0aa8dc42493fa622aaa87c814fd8e67a6fb3fa1f1e5d952d"),
    Package(name="httpx2", version="2.16.0", sha256="0d54c98128e8f2ff7fbbde0fca8c359e5f032edeaaaac443cdb7f31f685bcd7f"),
    Package(
        name="watchfiles", version="1.3.0", sha256="99aee4a07847c06820765fd7b1b49ceac4f3f711ccb7d104655a33231de1c207"
    ),
    Package(
        name="httpcore2", version="2.16.0", sha256="c72cd0413563b17e1cd8354677dff887466f8ec2551679759e93126534428643"
    ),
)
#: The maintainer's own Python projects listed on https://bernat.tech/oss that the list above misses, so the rules are
#: checked against code written in the style they enforce.
_MAINTAINED: Final[tuple[Package, ...]] = (
    Package(
        name="virtualenv", version="21.15.5", sha256="fb869ea32930530ccc87549877b53268c2a4d4a7d7f2c384042eca05170350e6"
    ),
    Package(name="tox", version="4.67.11", sha256="9040a61d57304159fd2cbd726469c2a7cd513a8c669069433392eab1a756f344"),
    Package(
        name="pyproject-api",
        version="1.12.4",
        sha256="79d0584a851fc945150715fa7871b0d72f48000e848b2298193d160cc18b3c89",
    ),
    Package(
        name="pytest-env", version="1.8.5", sha256="992c37efdc1bde2f0783975080ac4cdcea85f36df9d6fe3a835edd01a7e06584"
    ),
    Package(
        name="pipdeptree", version="4.2.14", sha256="f1232630665aec3401328106cb8bb1f3f89cdacac94b3a7a7ba8b4ea21b3c612"
    ),
    Package(
        name="sphinx-autodoc-typehints",
        version="3.17.5",
        sha256="8eaad05b05b765b3bb4c2cf1c1410eacce9d31a156d92cac9eb21a0a9099c90d",
    ),
    Package(
        name="pre-commit-uv", version="4.4.2", sha256="a29a29fe2e0b294ba5a75e28b0ca7804eb04609129603b1897411e44aed8701c"
    ),
    Package(
        name="pyproject-fmt",
        version="2.30.6",
        sha256="ffd8fb8c245de951a8a944d00ceb536f55f080d70e38e7c8f86bb114f63e4574",
    ),
    Package(
        name="tox-toml-fmt", version="1.11.3", sha256="90f068ff7aedf6bcef04114922286e3a08d089af1a4432da386039edb5252f18"
    ),
    Package(
        name="pytest-print", version="1.4.2", sha256="30b6f708929b42092cfcd3e3202a4a8ec2aa255a13280350153bb07aa42981fd"
    ),
    Package(name="tox-gh", version="1.9.2", sha256="3006f4f7c1bd36ee1579b9d8f842daec036d14a418f74baffc56043fab086798"),
    Package(
        name="sphinx-argparse-cli",
        version="1.24.2",
        sha256="be83d0736843a95b1a513fb81e479d5aa057644aeb7ff6e40cd1f9a4ece254cc",
    ),
    Package(
        name="python-discovery",
        version="1.9.4",
        sha256="c01b45fe515c1d268b57c57d554bb0e1d320b8e9eb5490bb2f4f577be26e4380",
    ),
    Package(
        name="devpi-process", version="1.3.3", sha256="5caeb8a15625bb92a91db7bc245bd138ca0cdb41667a8801a14734fe10bacde6"
    ),
    Package(
        name="tox-ini-fmt", version="1.10.2", sha256="5200404a511a653dfc276fb7ab75a44706d466cf32429e9ee69348d35f3092c6"
    ),
    Package(
        name="bump-deps-index",
        version="1.19.1",
        sha256="cda735a20a70fe5cc05e717170d16c2e4337df4c5a5b6cd6862c20e419ad0061",
    ),
    Package(
        name="pypi-changes", version="1.7.5", sha256="ae8cd1f472aa1913dd69fe725babb5c8d6d98f348eb21021f4103d246d16edaa"
    ),
    Package(
        name="simplint", version="0.6.0", sha256="ff8112301dabaec074068f10fc66aa9457a9f13dde2c4d2a8e7cc2adeb07697c"
    ),
    Package(
        name="mdformat-hugo", version="0.3.5", sha256="bff61ca4800bad9746eefd177e275ece4d30fc504800cc45bca2862779965e4e"
    ),
    Package(name="xml-fmt", version="1.2.2", sha256="cf8961ccaa5d57812a090e04279bdb6d1ce24353c349ceab3bad23a4b461836c"),
    Package(
        name="turbohtml", version="1.15.8", sha256="71807c2e2954697e46a49446e0cac6e5f2cff208d684398317ea492a6112326e"
    ),
    Package(
        name="mdformat-zola", version="1.0.3", sha256="33fdb34b17e5e764308d8f3e31496659a9307e6f43aa9af2f986ffde2b335a0b"
    ),
    Package(name="build", version="1.6.1", sha256="51cc11666391ab6f092070437ac747002ff46f3e4113a3622177ee6b488bfc53"),
    Package(name="pipx", version="1.18.14", sha256="17951d60a01c9235fb68f0031f01ef41f3bd87a7488b2698ca4d68b1723ecdde"),
)
PACKAGES: Final[tuple[Package, ...]] = (*_TOP, *_MAINTAINED)


def main(arguments: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, default=Path(".tox") / "ecosystem", help="keeps the downloads")
    parser.add_argument("--simplint", default="simplint", help="the simplint executable to check")
    parser.add_argument(
        "--target-version",
        type=minor_version,
        metavar="3.X",
        help="check the fixed files against this CPython and simplint's rules for that release, not the running one",
    )
    parser.add_argument("names", nargs="*", help="projects to check, all when none are given")
    options = parser.parse_args(arguments, namespace=_Options())
    names = options.names
    if unknown := set(names) - {package.name for package in PACKAGES}:
        msg = f"not pinned: {', '.join(sorted(unknown))}"
        raise ValueError(msg)
    target = resolve(options.target_version) if options.target_version is not None else None
    selected = [package for package in PACKAGES if not names or package.name in names]
    archives = [download(Pinned(url=package.url, sha256=package.sha256), options.cache) for package in selected]
    failed = False
    # Each project unpacks into a folder of its own, so they run side by side; `map` hands the results back in the
    # pinned order, which keeps the report the same from run to run.
    with ProcessPoolExecutor() as pool:
        checked = pool.map(check, selected, archives, repeat(options.simplint), repeat(target))
        for package, problems in zip(selected, checked, strict=True):
            failed |= bool(problems)
            print(f"{package.name} {package.version}: {'; '.join(problems) or 'ok'}", flush=True)
    return int(failed)


class _Options(argparse.Namespace):
    cache: Path
    simplint: str
    target_version: int | None
    names: list[str]


_CONFLICTING_PATTERN: Final[re.Pattern[str]] = re.compile(r"\(`([A-Z]+[0-9]+)`\)")


def check(package: Package, archive: Path, simplint: str, target: Interpreter | None) -> list[str]:
    # Run the documented pipeline on a project and check that one round of it reaches a fixed point.
    #
    # The pipeline is ``ruff check --fix --unsafe-fixes``, ``simplint check --fix --unsafe-fixes`` and ``ruff format``.
    # The baseline runs the same commands without simplint.
    #
    # :param package: the project, for its name in messages
    # :param archive: its source distribution
    # :param simplint: the simplint executable
    # :param target: the CPython the fixed files must compile under, and the release simplint checks for; None checks
    #     with the running interpreter's parser and the release the project's requires-python gives
    # :returns: one message per problem found, empty when the project passes
    with TemporaryDirectory(prefix=f"{package.name}-") as folder:
        work = _workspace(Path(folder), archive)
        report = ["--output-format", "concise", "."]
        for tree in (work.original, work.fixed):
            work.run(["ruff", "check", "--fix", "--unsafe-fixes", "--exit-zero", "--quiet", "."], tree)
        # A file Ruff cannot read either, such as one in another encoding, is not simplint's failure.
        unreadable = set(_io_errors(work.run(["ruff", "check", "--no-fix", "--exit-zero", *report], work.fixed)))
        simplint_check = [simplint, "check"]
        if target is not None:
            simplint_check += ["--target-version", f"py3{target.minor}"]
        simplint_check, checked = _checked(simplint_check, report, work)
        problems = [f"check: {line}" for line in _failures(checked, unreadable)]
        fixing = work.run([*simplint_check, "--fix-only", "--unsafe-fixes", *report], work.fixed)
        problems += [f"fix: {line}" for line in _failures(fixing, unreadable)]
        # Rules Ruff warns conflict with its formatter for this project cannot hold after `ruff format`, with or without
        # simplint, as Ruff documents; their findings say nothing about simplint.
        formatting = work.run(["ruff", "format", "--check", "."], work.original)
        conflicting = _CONFLICTING_PATTERN.findall(formatting.stderr)
        unaffected = ["--ignore", ",".join(conflicting)] if conflicting else []
        # Formatting splits a line and can leave its pragma comment after the closing bracket, where it no longer
        # covers the line it was written for; that happens to the project with or without simplint, as Ruff matches
        # suppressions the same way.
        stranded_before = _stranded_pragmas(work.original)
        for tree in (work.original, work.fixed):
            work.run(["ruff", "format", "--quiet", "."], tree)
        stranding = {
            path for path, count in _stranded_pragmas(work.original).items() if count > stranded_before.get(path, 0)
        }
        broken = sorted(_compiling(work.original, target) - _compiling(work.fixed, target))
        problems += [f"{path} no longer parses" for path in broken]
        # The project's own settings give the target version, and so whether annotations are evaluated lazily.
        undefined = ["--select", "F821,F811,F823", "--ignore-noqa", "--config", "lint.per-file-ignores = {}"]
        problems += [f"{path}: new {code}" for path, code in _new_findings(work, undefined)]
        problems += [f"{path}: new {code}" for path, code in _new_findings(work, unaffected)]
        problems += _second_round(simplint_check, work, unaffected, stranding)
        return problems


def _workspace(folder: Path, archive: Path) -> _Workspace:
    unpacked = folder / "original"
    with tarfile.open(archive) as tar:
        tar.extractall(unpacked, filter="data")
    # Run from the project's own directory, as a user would: Ruff, and simplint with it, look for the
    # `requires-python` that sets the target version from where it runs.
    original = next(unpacked.iterdir())
    fixed = folder / "fixed" / original.name
    shutil.copytree(original, fixed)
    (folder / "config").mkdir()
    # Ruff and simplint read the user's configuration from `XDG_CONFIG_HOME`, and an environment override of `RUFF_*`
    # settings, which would make a local run judge the projects differently from CI. A project's own `cache-dir` can
    # point outside the workspace, so `RUFF_CACHE_DIR` pins Ruff's cache there, and simplint keeps its own in it.
    inherited = {key: value for key, value in os.environ.items() if not key.startswith("RUFF_")}
    environment = inherited | {"XDG_CONFIG_HOME": str(folder / "config"), "RUFF_CACHE_DIR": str(folder / "cache")}
    return _Workspace(original=original, fixed=fixed, environment=environment)


@dataclass(frozen=True, slots=True, kw_only=True)
class _Workspace:
    # The project as it unpacked, the copy the fixes go to, and the environment the tools run in.
    original: Path
    fixed: Path
    environment: dict[str, str]

    def run(self, command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(command, cwd=cwd, env=self.environment, capture_output=True, text=True, check=False)


_IO_ERRORS_PATTERN: Final[re.Pattern[str]] = re.compile(r":\d+:\d+: (E902 |io-error: )")


# Ruff names the rule `E902`, or `io-error` when preview shows rules by name.
def _io_errors(completed: subprocess.CompletedProcess[str]) -> list[str]:
    return [line for line in completed.stdout.splitlines() if _IO_ERRORS_PATTERN.search(line)]


_REFUSED_RULE_PATTERN: Final[re.Pattern[str]] = re.compile(r"ignore (\S+) in `\[tool\.simplint\]`")


# simplint refuses to run when the project's Ruff settings work against one of its rules; a user of the project would
# turn that rule off, as the error says, so the check runs once more with that rule ignored.
def _checked(
    simplint_check: list[str], report: list[str], work: _Workspace
) -> tuple[list[str], subprocess.CompletedProcess[str]]:
    checked = work.run([*simplint_check, *report], work.fixed)
    if not (refused := _REFUSED_RULE_PATTERN.findall(checked.stderr)):
        return simplint_check, checked
    retried = [*simplint_check, "--ignore", ",".join(refused)]
    return retried, work.run([*retried, *report], work.fixed)


# As with `ruff check`, a report exits 1 and only a run that could not finish exits 2; a file simplint cannot read or a
# fix it had to revert shows up as an `io-error` report or an error on standard error.
def _failures(completed: subprocess.CompletedProcess[str], unreadable: set[str]) -> list[str]:
    failures = [line for line in completed.stderr.splitlines() if line.startswith(("error", "simplint failed"))]
    failures += [line for line in _io_errors(completed) if line not in unreadable]
    if completed.returncode > 1 and not failures:
        failures.append(completed.stderr.strip())
    return failures


_STRANDED_PRAGMAS_PATTERN: Final[re.Pattern[str]] = re.compile(
    r"""
        ^\s*[)\]}]+[,:]?  # closing syntax at the start of a line
        \s*\#\s*          # comment marker
        (?:noqa|type:|pyright:|ruff:|simplint:)  # suppression directive
        """,
    re.MULTILINE | re.VERBOSE,
)


def _stranded_pragmas(root: Path) -> dict[str, int]:
    return {
        path.relative_to(root).as_posix(): len(_STRANDED_PRAGMAS_PATTERN.findall(path.read_text(errors="replace")))
        for path in root.rglob("*.py")
    }


#: Compiles the files of a tree, named by a JSON list of relative paths on stdin, and prints the ones that compile; one
#: subprocess per tree runs it over every file.
_COMPILE_TREE: Final[str] = """\
import json
import sys
import warnings

warnings.simplefilter("ignore")
compiling = []
for path in json.load(sys.stdin):
    with open(path, "rb") as handle:
        source = handle.read()
    try:
        compile(source, path, "exec", dont_inherit=True)
    except SyntaxError:
        continue
    compiling.append(path)
json.dump(compiling, sys.stdout)
"""


# The running parser knows only this interpreter's grammar, so an older target compiles the files itself.
def _compiling(root: Path, target: Interpreter | None) -> set[str]:
    paths = {path.relative_to(root).as_posix() for path in root.rglob("*.py")}
    if target is None:
        return {path for path in paths if _parses(root / path)}
    compiling = subprocess.run(
        [target.path, "-I", "-c", _COMPILE_TREE],
        cwd=root,
        input=json.dumps(sorted(paths)),
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return set(json.loads(compiling))


def _parses(path: Path) -> bool:
    # Invalid escape sequences in old projects warn, which is not what the check is about.
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", SyntaxWarning)
        try:
            ast.parse(path.read_bytes())
        except SyntaxError:
            return False
    return True


# Renames move a name between lines, so findings are compared by count per file and code rather than by location.
def _new_findings(work: _Workspace, options: list[str]) -> list[tuple[str, str]]:
    before, after = _findings(work, work.original, options), _findings(work, work.fixed, options)
    return sorted(key for key, count in after.items() if count > before[key])


def _findings(work: _Workspace, root: Path, options: list[str]) -> Counter[tuple[str, str]]:
    report = json.loads(
        work.run(["ruff", "check", "--no-fix", *options, "--exit-zero", "--output-format", "json", "."], root).stdout
    )
    return Counter(
        (Path(finding["filename"]).relative_to(root.resolve()).as_posix(), finding["code"] or "invalid-syntax")
        for finding in report
    )


# What a second round of the pipeline changes in the fixed tree, which it must not.
def _second_round(simplint_check: list[str], work: _Workspace, unaffected: list[str], stranding: set[str]) -> list[str]:
    problems = []
    # Ruff alone does not always settle in one round, so a second round counts only in the files where Ruff settles the
    # baseline: in the others Ruff hands simplint a tree the pipeline never settles either.
    ruff_again = ["ruff", "check", "--fix-only", "--unsafe-fixes", "--diff", "--quiet", *unaffected, "."]
    unsettled = _changed(work.run(ruff_again, work.original).stdout)
    if _changed(work.run(ruff_again, work.fixed).stdout) - unsettled:
        problems.append("a second round of ruff check --fix changed the code")
    simplint_again = [*simplint_check, "--unsafe-fixes", "--diff", "."]
    if _changed(work.run(simplint_again, work.fixed).stdout) - unsettled - stranding:
        problems.append("a second round of simplint check --fix changed the code")
    if work.run(["ruff", "format", "--check", "--quiet", "."], work.fixed).returncode == 1:
        problems.append("ruff format left the code unformatted")
    return problems


_DIFF_FILE_PATTERN: Final[re.Pattern[str]] = re.compile(r"^--- (\S+)$", re.MULTILINE)


def _changed(diff: str) -> set[str]:
    return set(_DIFF_FILE_PATTERN.findall(diff))


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))


__all__ = [
    "PACKAGES",
    "Package",
    "check",
    "main",
]
