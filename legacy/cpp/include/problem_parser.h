#pragma once
#include <istream>
#include <vector>

#include "problem.h"

class ProblemParser {
public:
    Problem Parse(std::istream& input) const;
};
