#pragma once

#include <string>

#include "problem.h"

class ProblemIO {
public:
    Problem ReadProblem(const std::string& archivePath) const;
    void WriteProblem(const std::string& archivePath, const Problem& p) const;
};
