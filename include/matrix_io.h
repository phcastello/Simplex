#pragma once

#include <string>

#include "matrix.h"

class MatrixIO {
public:
    Matrix ReadMatrix(const std::string& archivePath) const;

    void WriteMatrix(const std::string& archivePath) const;
};
