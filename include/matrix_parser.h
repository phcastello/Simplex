#pragma once
#include <istream>
#include <vector>

#include "matrix.h"

class MatrixParser {
public:
    Matrix Parse(std::istream& input) const;
};
