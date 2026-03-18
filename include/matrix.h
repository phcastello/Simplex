#pragma once
#include <vector>
#include <cmath>
#include <stdexcept>

class Matrix{
private:
    std::vector<std::vector<double>> data_;

    Matrix makeMinorMatrix(std::size_t linhaRemovida, std::size_t colunaRemovida) const;
    
    // Não use, só existe pra recordação
    Matrix makeMinorMatrixGAMBIARRA(std::size_t linhaRemovida, std::size_t colunaRemovida) const;
public:
    // Construtores
    Matrix() = default;
    Matrix(std::size_t rows, std::size_t cols, double initialValue = 0.0);
    Matrix(const std::vector<std::vector<double>>& data);
    Matrix(std::vector<std::vector<double>>&& data);

    // Metodos auxiliares
    std::size_t rows() const;
    std::size_t cols() const;
    bool empty() const;
    bool isRectangular() const;
    bool isSquare() const;
    void push_back(std::vector<double> value);
    double& at(std::size_t i, std::size_t j);
    const double& at(std::size_t i, std::size_t j) const;
    
    // Metodos de cálculo
    // Calcula o determinante por laplace
    double determinant() const;
    Matrix inverse() const;
};
