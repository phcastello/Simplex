#include "problem_io.h"

#include <fstream>
#include <stdexcept>

#include "problem_parser.h"

Problem ProblemIO::ReadProblem(const std::string& archivePath) const {
    std::ifstream problemFile(archivePath);
    if (!problemFile) {
        throw std::runtime_error("Erro ao abrir o arquivo " + archivePath + "\n");
    }

    ProblemParser parser;
    return parser.Parse(problemFile);
}

void ProblemIO::WriteProblem(const std::string& archivePath, const Problem& p) const {
    std::ofstream file(archivePath);
    if (!file) {
        throw std::runtime_error("Erro ao abrir o arquivo " + archivePath + "\n");
    }
    file << p.toString();
}
