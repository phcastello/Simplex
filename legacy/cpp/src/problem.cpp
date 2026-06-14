#include "problem.h"

#include <algorithm>
#include <cmath>
#include <iomanip>
#include <sstream>
#include <vector>

namespace {

// Formats a double removing trailing zeros.
std::string fmtNum(double v) {
    std::ostringstream oss;
    oss << std::fixed << std::setprecision(10) << v;
    std::string s = oss.str();
    if (s.find('.') != std::string::npos) {
        while (!s.empty() && s.back() == '0') s.pop_back();
        if (!s.empty() && s.back() == '.') s.pop_back();
    }
    return s;
}

// Appends a term to the stream.
// isFirst controls whether to prefix with sign vs. " + "/" - ".
void appendTerm(std::ostream& ss, const Problem::Term& t, bool isFirst) {
    double absCoeff = std::abs(t.coeff);
    bool positive = (t.coeff >= 0.0);

    if (isFirst) {
        if (!positive) ss << "-";
        if (absCoeff != 1.0) ss << fmtNum(absCoeff);
    } else {
        ss << (positive ? " + " : " - ");
        if (absCoeff != 1.0) ss << fmtNum(absCoeff);
    }
    ss << "x_" << t.varIndex;
}

} // anonymous namespace

// ---------------------------------------------------------------------------

Problem Problem::makeNormalProblem() const {
    Problem normal;
    normal.sense_ = sense_;
    normal.objective_ = objective_;

    // Find the highest variable index currently in use
    int maxIdx = 0;
    for (const auto& t : objective_)
        maxIdx = std::max(maxIdx, t.varIndex);
    for (const auto& c : constraints_)
        for (const auto& t : c.terms)
            maxIdx = std::max(maxIdx, t.varIndex);

    // Copy original variable bounds
    for (const auto& [idx, rel] : varBounds_)
        normal.varBounds_[idx] = rel;

    // Convert each constraint to equality, adding a slack variable when needed
    for (const auto& c : constraints_) {
        if (c.relation == Relation::EQ) {
            normal.constraints_.push_back(c);
        } else {
            int slackIdx = ++maxIdx;
            Constraint nc = c;
            nc.terms.push_back({slackIdx, 1.0});
            nc.relation = Relation::EQ;
            normal.constraints_.push_back(nc);

            // <= constraint: slack >= 0
            // >= constraint: slack <= 0
            normal.varBounds_[slackIdx] =
                (c.relation == Relation::LE) ? Relation::GE : Relation::LE;
        }
    }

    // Propagate parsing/normalization warnings
    for (const auto& w : warnings_)
        normal.warnings_.push_back(w);

    return normal;
}

std::string Problem::toString() const {
    std::ostringstream ss;

    // --- Objective ---
    ss << (sense_ == Sense::Max ? "max" : "min") << " z = ";
    for (size_t i = 0; i < objective_.size(); i++)
        appendTerm(ss, objective_[i], i == 0);
    ss << "\n";

    // --- Constraints ---
    for (const auto& c : constraints_) {
        ss << "    ";
        for (size_t i = 0; i < c.terms.size(); i++)
            appendTerm(ss, c.terms[i], i == 0);

        const char* relStr = (c.relation == Relation::LE) ? "<="
                           : (c.relation == Relation::GE) ? ">=" : "=";
        ss << " " << relStr << " " << fmtNum(c.rhs) << "\n";
    }

    // --- Bounds: <= 0 first, then >= 0, asc ---
    std::vector<int> leVars, geVars;
    for (const auto& [idx, rel] : varBounds_) {
        if (rel == Relation::LE) leVars.push_back(idx);
        else                     geVars.push_back(idx);
    }

    if (!leVars.empty()) {
        ss << "    ";
        for (size_t i = 0; i < leVars.size(); i++) {
            if (i > 0) ss << ", ";
            ss << "x_" << leVars[i];
        }
        ss << " <= 0\n";
    }

    if (!geVars.empty()) {
        ss << "    ";
        for (size_t i = 0; i < geVars.size(); i++) {
            if (i > 0) ss << ", ";
            ss << "x_" << geVars[i];
        }
        ss << " >= 0\n";
    }

    return ss.str();
}
