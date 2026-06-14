#include "problem_parser.h"

#include <algorithm>
#include <cctype>
#include <set>
#include <sstream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

std::string trim(const std::string& s) {
    size_t start = s.find_first_not_of(" \t\r\n");
    if (start == std::string::npos) return "";
    size_t end = s.find_last_not_of(" \t\r\n");
    return s.substr(start, end - start + 1);
}

// Finds the relation operator in a line.
// Fills opStart (index of first op char) and opEnd (index after last op char).
// isStrict is set true when < or > (strict) was found, converted to LE/GE.
// Throws if no operator is found.
Problem::Relation findRelation(const std::string& line,
                               size_t& opStart, size_t& opEnd,
                               bool& isStrict) {
    isStrict = false;

    // Check two-char operators first
    for (size_t i = 0; i + 1 < line.size(); i++) {
        if (line[i] == '<' && line[i + 1] == '=') {
            opStart = i; opEnd = i + 2;
            return Problem::Relation::LE;
        }
        if (line[i] == '>' && line[i + 1] == '=') {
            opStart = i; opEnd = i + 2;
            return Problem::Relation::GE;
        }
    }

    // Single-char operators
    for (size_t i = 0; i < line.size(); i++) {
        if (line[i] == '<') {
            opStart = i; opEnd = i + 1;
            isStrict = true;
            return Problem::Relation::LE;
        }
        if (line[i] == '>') {
            opStart = i; opEnd = i + 1;
            isStrict = true;
            return Problem::Relation::GE;
        }
        if (line[i] == '=') {
            opStart = i; opEnd = i + 1;
            return Problem::Relation::EQ;
        }
    }

    throw std::runtime_error("No relation operator found in: \"" + line + "\"");
}

// Returns true if s is exactly "x_<digits>" (no surrounding spaces).
bool isJustVarName(const std::string& s) {
    if (s.size() < 3) return false;
    if (s[0] != 'x' || s[1] != '_') return false;
    for (size_t i = 2; i < s.size(); i++) {
        if (!std::isdigit(static_cast<unsigned char>(s[i]))) return false;
    }
    return true;
}

// Detects whether a (trimmed) line is a bound declaration.
// Bound: comma in LHS, or single-var "x_n <=/>=  0".
bool isBoundLine(const std::string& line) {
    if (line.find(',') != std::string::npos) return true;

    size_t opStart = 0, opEnd = 0;
    bool isStrict = false;
    Problem::Relation rel;
    try {
        rel = findRelation(line, opStart, opEnd, isStrict);
    } catch (...) {
        return false;
    }

    // EQ operator is not used for bounds
    if (rel == Problem::Relation::EQ) return false;

    std::string lhs = trim(line.substr(0, opStart));
    std::string rhs = trim(line.substr(opEnd));

    return isJustVarName(lhs) && (rhs == "0");
}

// Parses a linear expression like "5x_1 + 4x_2 - x_3".
// Returns terms sorted in insertion order.
std::vector<Problem::Term> parseLinearExpression(const std::string& expr) {
    std::vector<Problem::Term> terms;
    size_t pos = 0;
    size_t n = expr.size();

    auto skipSpaces = [&]() {
        while (pos < n && std::isspace(static_cast<unsigned char>(expr[pos]))) pos++;
    };

    while (pos < n) {
        skipSpaces();
        if (pos >= n) break;

        // Sign
        double sign = 1.0;
        if (expr[pos] == '+') { sign = 1.0; pos++; }
        else if (expr[pos] == '-') { sign = -1.0; pos++; }

        skipSpaces();
        if (pos >= n) throw std::runtime_error("Unexpected end of expression");

        // Coefficient (optional)
        double coeff = 1.0;
        size_t start = pos;
        while (pos < n && (std::isdigit(static_cast<unsigned char>(expr[pos])) || expr[pos] == '.'))
            pos++;
        if (pos > start)
            coeff = std::stod(expr.substr(start, pos - start));

        skipSpaces();

        // Expect 'x'
        if (pos >= n || expr[pos] != 'x')
            throw std::runtime_error("Expected variable 'x_<n>' in expression: \"" + expr + "\"");
        pos++;

        // Expect '_'
        if (pos >= n || expr[pos] != '_')
            throw std::runtime_error("Expected '_' after 'x' in: \"" + expr + "\"");
        pos++;

        // Variable index
        start = pos;
        while (pos < n && std::isdigit(static_cast<unsigned char>(expr[pos]))) pos++;
        if (pos == start)
            throw std::runtime_error("Expected digits after 'x_' in: \"" + expr + "\"");
        int idx = std::stoi(expr.substr(start, pos - start));

        terms.push_back({idx, sign * coeff});
    }

    return terms;
}

void parseObjectiveLine(const std::string& line, Problem& p) {
    size_t pos = 0;
    size_t n = line.size();

    auto skipSpaces = [&]() {
        while (pos < n && std::isspace(static_cast<unsigned char>(line[pos]))) pos++;
    };

    skipSpaces();

    // Sense word
    size_t start = pos;
    while (pos < n && !std::isspace(static_cast<unsigned char>(line[pos]))) pos++;
    std::string senseStr = line.substr(start, pos - start);
    std::transform(senseStr.begin(), senseStr.end(), senseStr.begin(),
                   [](unsigned char c) { return std::tolower(c); });

    if (senseStr == "max") {
        p.setSense(Problem::Sense::Max);
    } else if (senseStr == "min") {
        p.setSense(Problem::Sense::Min);
    } else {
        throw std::runtime_error("Expected 'max' or 'min', got: \"" + senseStr + "\"");
    }

    skipSpaces();

    // Objective variable name 'z'
    if (pos >= n || line[pos] != 'z')
        throw std::runtime_error("Expected 'z' after sense");
    pos++;

    skipSpaces();

    // '='
    if (pos >= n || line[pos] != '=')
        throw std::runtime_error("Expected '=' after 'z'");
    pos++;

    skipSpaces();

    std::string exprStr = line.substr(pos);
    p.setObjective(parseLinearExpression(trim(exprStr)));
}

void parseConstraintLine(const std::string& line, Problem& p) {
    size_t opStart = 0, opEnd = 0;
    bool isStrict = false;
    Problem::Relation rel = findRelation(line, opStart, opEnd, isStrict);

    if (isStrict) {
        char op = (rel == Problem::Relation::LE) ? '<' : '>';
        p.addWarning(std::string("Strict operator '") + op +
                     "' converted to non-strict in: \"" + line + "\"");
    }

    std::string lhs = trim(line.substr(0, opStart));
    std::string rhs = trim(line.substr(opEnd));

    auto terms = parseLinearExpression(lhs);

    double rhsVal;
    try {
        rhsVal = std::stod(rhs);
    } catch (const std::invalid_argument&) {
        throw std::runtime_error("Invalid RHS in constraint: \"" + rhs + "\"");
    }

    Problem::Constraint c;
    c.terms = std::move(terms);
    c.relation = rel;
    c.rhs = rhsVal;
    p.addConstraint(std::move(c));
}

void parseBoundLine(const std::string& line, Problem& p) {
    size_t opStart = 0, opEnd = 0;
    bool isStrict = false;
    Problem::Relation rel = findRelation(line, opStart, opEnd, isStrict);

    if (isStrict) {
        char op = (rel == Problem::Relation::LE) ? '<' : '>';
        p.addWarning(std::string("Strict operator '") + op +
                     "' converted to non-strict in bound: \"" + line + "\"");
    }

    if (rel == Problem::Relation::EQ)
        throw std::runtime_error("Bound operator cannot be '=': \"" + line + "\"");

    std::string lhs = trim(line.substr(0, opStart));
    std::string rhs = trim(line.substr(opEnd));

    // RHS must be 0
    try {
        double rhsVal = std::stod(rhs);
        if (rhsVal != 0.0)
            throw std::runtime_error("Bound RHS must be 0, got: \"" + rhs + "\"");
    } catch (const std::invalid_argument&) {
        throw std::runtime_error("Invalid bound RHS: \"" + rhs + "\"");
    }

    // Split LHS on commas
    std::stringstream ss(lhs);
    std::string token;
    while (std::getline(ss, token, ',')) {
        std::string varStr = trim(token);
        if (!isJustVarName(varStr))
            throw std::runtime_error("Invalid variable in bound: \"" + varStr + "\"");

        int idx = std::stoi(varStr.substr(2));

        if (p.varBounds().count(idx) > 0)
            throw std::runtime_error(
                "Variable x_" + std::to_string(idx) + " appears twice in bounds");

        p.setVarBound(idx, rel);
    }
}

// After all lines are parsed, validate and fill in defaults for bounds.
void validateBounds(Problem& p) {
    // Collect all variable indices used in the problem
    std::set<int> usedVars;
    for (const auto& t : p.objective()) usedVars.insert(t.varIndex);
    for (const auto& c : p.constraints())
        for (const auto& t : c.terms) usedVars.insert(t.varIndex);

    if (p.varBounds().empty()) {
        for (int idx : usedVars) p.setVarBound(idx, Problem::Relation::LE);
        p.addWarning(
            "No variable bounds specified; assuming x_i <= 0 for all original variables");
        return;
    }

    // All used variables must appear in bounds exactly once
    for (int idx : usedVars) {
        if (p.varBounds().count(idx) == 0)
            throw std::runtime_error(
                "Partial bounds: x_" + std::to_string(idx) +
                " is used in the problem but missing from bounds");
    }

    // All bound variables must be used in the problem
    for (const auto& [idx, rel] : p.varBounds()) {
        if (usedVars.count(idx) == 0)
            throw std::runtime_error(
                "Bound declared for x_" + std::to_string(idx) +
                " but that variable is not used in the problem");
    }
}

} // anonymous namespace

Problem ProblemParser::Parse(std::istream& input) const {
    Problem p;
    std::string line;
    bool objectiveParsed = false;
    bool inBoundSection = false;

    while (std::getline(input, line)) {
        std::string trimmed = trim(line);
        if (trimmed.empty()) continue;

        if (!objectiveParsed) {
            parseObjectiveLine(trimmed, p);
            objectiveParsed = true;
        } else if (inBoundSection) {
            if (!isBoundLine(trimmed))
                throw std::runtime_error(
                    "Constraint line found after bound section: \"" + trimmed + "\"");
            parseBoundLine(trimmed, p);
        } else if (isBoundLine(trimmed)) {
            inBoundSection = true;
            parseBoundLine(trimmed, p);
        } else {
            parseConstraintLine(trimmed, p);
        }
    }

    if (!objectiveParsed)
        throw std::runtime_error("Empty input: no objective found");

    validateBounds(p);

    return p;
}
