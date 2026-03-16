#pragma once
#include <utility>
#include <vector>

class Matrix : public std::vector<std::vector<double>>{
public:
    using Base = std::vector<std::vector<double>>;
    using Base::Base;

    Matrix(const Base& v) : Base(v) {}
    Matrix(Base&& v) : Base(std::move(v)) {}

    bool isSquare() const;
    double determinant() const;
    Matrix inverse() const;
};
