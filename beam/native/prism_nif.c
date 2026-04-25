/*
 * prism_nif.c — Bridges Fortran prism operations to Erlang NIF API.
 *
 * Handles the row-major (Erlang lists) <-> column-major (Fortran arrays)
 * transpose transparently.
 */

#include <erl_nif.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>

#define MAX_DIM 1024

/* Fortran symbols (iso_c_binding, no mangling) */
extern void prism_preview(int n, const double *projection, const double *source,
                          double *focus, int *matched);
extern void prism_review(int n, const double *projection, const double *focus,
                         double *result);
extern void prism_modify(int n, const double *projection, const double *source,
                         const double *transform, double *result);
extern void prism_compose(int n, const double *p1, const double *p2,
                          double *composed);

/* --- Conversion helpers --- */

static int list_to_doubles(ErlNifEnv *env, ERL_NIF_TERM list,
                           double *out, int expected) {
    ERL_NIF_TERM head, tail;
    int i = 0;
    double val;
    long ival;

    tail = list;
    while (i < expected && enif_get_list_cell(env, tail, &head, &tail)) {
        if (enif_get_double(env, head, &val)) {
            out[i] = val;
        } else if (enif_get_long(env, head, &ival)) {
            out[i] = (double)ival;
        } else {
            return -1;
        }
        i++;
    }
    return (i == expected) ? 0 : -1;
}

static ERL_NIF_TERM doubles_to_list(ErlNifEnv *env, const double *arr, int n) {
    ERL_NIF_TERM list = enif_make_list(env, 0);
    for (int i = n - 1; i >= 0; i--) {
        list = enif_make_list_cell(env,
                                   enif_make_double(env, arr[i]),
                                   list);
    }
    return list;
}

static int matrix_to_col_major(ErlNifEnv *env, ERL_NIF_TERM matrix,
                               double *out, int n) {
    ERL_NIF_TERM row_term, tail;
    double row[MAX_DIM];
    int r = 0;

    tail = matrix;
    while (r < n && enif_get_list_cell(env, tail, &row_term, &tail)) {
        if (list_to_doubles(env, row_term, row, n) != 0) return -1;
        for (int c = 0; c < n; c++) {
            out[r + c * n] = row[c];
        }
        r++;
    }
    return (r == n) ? 0 : -1;
}

static ERL_NIF_TERM col_major_to_lists(ErlNifEnv *env, const double *mat, int n) {
    ERL_NIF_TERM rows = enif_make_list(env, 0);
    double row[MAX_DIM];

    for (int r = n - 1; r >= 0; r--) {
        for (int c = 0; c < n; c++) {
            row[c] = mat[r + c * n];
        }
        rows = enif_make_list_cell(env, doubles_to_list(env, row, n), rows);
    }
    return rows;
}

/* --- NIF functions --- */

static ERL_NIF_TERM nif_prism_preview(ErlNifEnv *env, int argc,
                                       const ERL_NIF_TERM argv[]) {
    int n;
    if (!enif_get_int(env, argv[0], &n) || n <= 0 || n > MAX_DIM)
        return enif_make_badarg(env);

    double *projection = malloc(n * n * sizeof(double));
    double *source = malloc(n * sizeof(double));
    double *focus = malloc(n * sizeof(double));
    int matched = 0;

    if (!projection || !source || !focus) {
        free(projection); free(source); free(focus);
        return enif_make_badarg(env);
    }

    if (matrix_to_col_major(env, argv[1], projection, n) != 0 ||
        list_to_doubles(env, argv[2], source, n) != 0) {
        free(projection); free(source); free(focus);
        return enif_make_badarg(env);
    }

    prism_preview(n, projection, source, focus, &matched);

    ERL_NIF_TERM result;
    if (matched) {
        result = enif_make_tuple2(env,
                    enif_make_atom(env, "ok"),
                    doubles_to_list(env, focus, n));
    } else {
        result = enif_make_tuple2(env,
                    enif_make_atom(env, "error"),
                    enif_make_atom(env, "nil"));
    }

    free(projection); free(source); free(focus);
    return result;
}

static ERL_NIF_TERM nif_prism_review(ErlNifEnv *env, int argc,
                                      const ERL_NIF_TERM argv[]) {
    int n;
    if (!enif_get_int(env, argv[0], &n) || n <= 0 || n > MAX_DIM)
        return enif_make_badarg(env);

    double *projection = malloc(n * n * sizeof(double));
    double *focus = malloc(n * sizeof(double));
    double *result_vec = malloc(n * sizeof(double));

    if (!projection || !focus || !result_vec) {
        free(projection); free(focus); free(result_vec);
        return enif_make_badarg(env);
    }

    if (matrix_to_col_major(env, argv[1], projection, n) != 0 ||
        list_to_doubles(env, argv[2], focus, n) != 0) {
        free(projection); free(focus); free(result_vec);
        return enif_make_badarg(env);
    }

    prism_review(n, projection, focus, result_vec);

    ERL_NIF_TERM result = doubles_to_list(env, result_vec, n);

    free(projection); free(focus); free(result_vec);
    return result;
}

static ERL_NIF_TERM nif_prism_modify(ErlNifEnv *env, int argc,
                                      const ERL_NIF_TERM argv[]) {
    int n;
    if (!enif_get_int(env, argv[0], &n) || n <= 0 || n > MAX_DIM)
        return enif_make_badarg(env);

    double *projection = malloc(n * n * sizeof(double));
    double *source = malloc(n * sizeof(double));
    double *transform = malloc(n * n * sizeof(double));
    double *result_vec = malloc(n * sizeof(double));

    if (!projection || !source || !transform || !result_vec) {
        free(projection); free(source); free(transform); free(result_vec);
        return enif_make_badarg(env);
    }

    if (matrix_to_col_major(env, argv[1], projection, n) != 0 ||
        list_to_doubles(env, argv[2], source, n) != 0 ||
        matrix_to_col_major(env, argv[3], transform, n) != 0) {
        free(projection); free(source); free(transform); free(result_vec);
        return enif_make_badarg(env);
    }

    prism_modify(n, projection, source, transform, result_vec);

    ERL_NIF_TERM result = doubles_to_list(env, result_vec, n);

    free(projection); free(source); free(transform); free(result_vec);
    return result;
}

static ERL_NIF_TERM nif_prism_compose(ErlNifEnv *env, int argc,
                                       const ERL_NIF_TERM argv[]) {
    int n;
    if (!enif_get_int(env, argv[0], &n) || n <= 0 || n > MAX_DIM)
        return enif_make_badarg(env);

    double *p1 = malloc(n * n * sizeof(double));
    double *p2 = malloc(n * n * sizeof(double));
    double *composed = malloc(n * n * sizeof(double));

    if (!p1 || !p2 || !composed) {
        free(p1); free(p2); free(composed);
        return enif_make_badarg(env);
    }

    if (matrix_to_col_major(env, argv[1], p1, n) != 0 ||
        matrix_to_col_major(env, argv[2], p2, n) != 0) {
        free(p1); free(p2); free(composed);
        return enif_make_badarg(env);
    }

    prism_compose(n, p1, p2, composed);

    ERL_NIF_TERM result = col_major_to_lists(env, composed, n);

    free(p1); free(p2); free(composed);
    return result;
}

static ErlNifFunc nif_funcs[] = {
    {"prism_preview", 3, nif_prism_preview, 0},
    {"prism_review",  3, nif_prism_review,  0},
    {"prism_modify",  4, nif_prism_modify,  0},
    {"prism_compose", 3, nif_prism_compose, 0},
};

ERL_NIF_INIT(conversation_prism_nif, nif_funcs, NULL, NULL, NULL, NULL)
