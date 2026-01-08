# SPDX-License-Identifier: Apache-2.0

import setuptools


def requirements():
    req = []
    with open("requirements.txt") as fd:
        for line in fd:
            line.strip()
            if not line.startswith("#"):
                req.append(line)
    return req


setuptools.setup(
    name="nipart",
    version="0.1.0",
    author="Gris Ge",
    author_email="fge@redhat.com",
    description="Python binding of Nipart",
    long_description="Python binding of Nipart",
    url="https://github.com/nipsor/nipart/",
    packages=setuptools.find_packages(),
    install_requires=requirements(),
    license="Apache-2.0",
    python_requires=">=3.10",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: Apache Software License",
        "Operating System :: POSIX :: Linux",
    ],
)
