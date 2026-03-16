cmake_minimum_required(VERSION 3.23)
project(simplex)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(simplex_lib
    src/aleatorizadorMatriz.cpp
    src/matrix.cpp
    src/matrix_parser.cpp
    src/matrix_io.cpp
)

target_include_directories(simplex_lib PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

add_executable(simplex main.cpp)

target_link_libraries(simplex PRIVATE simplex_lib)
