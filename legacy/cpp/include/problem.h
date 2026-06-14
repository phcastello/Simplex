#pragma once
#include <map>
#include <string>
#include <vector>

class Problem {
public:
    enum class Sense { Max, Min };
    enum class Relation { LE, GE, EQ };

    struct Term {
        int varIndex;
        double coeff;
    };

    struct Constraint {
        std::vector<Term> terms;
        Relation relation;
        double rhs;
    };

private:
    Sense sense_ = Sense::Max;
    std::vector<Term> objective_;
    std::vector<Constraint> constraints_;
    std::map<int, Relation> varBounds_;
    std::vector<std::string> warnings_;

public:
    void setSense(Sense s) { sense_ = s; }
    void setObjective(std::vector<Term> obj) { objective_ = std::move(obj); }
    void addConstraint(Constraint c) { constraints_.push_back(std::move(c)); }
    void setVarBound(int varIdx, Relation rel) { varBounds_[varIdx] = rel; }
    void addWarning(std::string w) { warnings_.push_back(std::move(w)); }

    Sense sense() const { return sense_; }
    const std::vector<Term>& objective() const { return objective_; }
    const std::vector<Constraint>& constraints() const { return constraints_; }
    const std::map<int, Relation>& varBounds() const { return varBounds_; }
    const std::vector<std::string>& warnings() const { return warnings_; }

    Problem makeNormalProblem() const;
    std::string toString() const;
};
