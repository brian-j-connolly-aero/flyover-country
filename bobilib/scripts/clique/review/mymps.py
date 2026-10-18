"""Independent, generic MPS + AUX reader (reviewer's own code).

Reads free/fixed-ish MPS by whitespace splitting (names in these files have no spaces).
Returns rows (name -> sense), objective row name, columns (col -> {row: coef}),
rhs (row -> value), bounds (col -> [lb, ub]), integer set, and the AUX data.
"""
import gzip
import math


def read_mps(path):
    op = gzip.open if path.endswith('.gz') else open
    rows = {}
    order_rows = []
    obj = None
    cols = {}
    col_order = []
    rhs = {}
    ranges = {}
    bounds = {}
    integer = set()
    objsense = 'MIN'
    section = None
    in_int = False
    with op(path, 'rt', encoding='latin-1') as f:
        for raw in f:
            line = raw.rstrip('\n')
            if not line.strip() or line.startswith('*'):
                continue
            if not line[0].isspace():
                tok = line.split()
                section = tok[0]
                if section == 'OBJSENSE' and len(tok) > 1:
                    objsense = tok[1]
                continue
            tok = line.split()
            if section == 'OBJSENSE':
                objsense = tok[0]
            elif section == 'ROWS':
                s, name = tok
                if s == 'N':
                    if obj is None:
                        obj = name
                    else:
                        raise ValueError('second N row ' + name)
                rows[name] = s
                order_rows.append(name)
            elif section == 'COLUMNS':
                if len(tok) >= 3 and tok[1] == "'MARKER'":
                    if tok[2] == "'INTORG'":
                        in_int = True
                    elif tok[2] == "'INTEND'":
                        in_int = False
                    else:
                        raise ValueError(line)
                    continue
                c = tok[0]
                if c not in cols:
                    cols[c] = {}
                    col_order.append(c)
                    bounds[c] = [0.0, math.inf]
                if in_int:
                    integer.add(c)
                pairs = tok[1:]
                if len(pairs) % 2:
                    raise ValueError(line)
                for i in range(0, len(pairs), 2):
                    r, v = pairs[i], float(pairs[i + 1])
                    if r not in rows:
                        raise ValueError('unknown row ' + r)
                    if r in cols[c]:
                        raise ValueError('dup entry ' + line)
                    cols[c][r] = v
            elif section == 'RHS':
                pairs = tok[1:] if len(tok) % 2 == 1 else tok
                for i in range(0, len(pairs), 2):
                    rhs[pairs[i]] = float(pairs[i + 1])
            elif section == 'RANGES':
                pairs = tok[1:] if len(tok) % 2 == 1 else tok
                for i in range(0, len(pairs), 2):
                    ranges[pairs[i]] = float(pairs[i + 1])
            elif section == 'BOUNDS':
                bt = tok[0]
                c = tok[2]
                v = float(tok[3]) if len(tok) > 3 else None
                if c not in bounds:
                    raise ValueError('bound on unknown col ' + c)
                if bt == 'BV':
                    bounds[c] = [0.0, 1.0]
                    integer.add(c)
                elif bt == 'UP':
                    bounds[c][1] = v
                elif bt == 'LO':
                    bounds[c][0] = v
                elif bt == 'FX':
                    bounds[c] = [v, v]
                elif bt == 'FR':
                    bounds[c] = [-math.inf, math.inf]
                elif bt == 'MI':
                    bounds[c][0] = -math.inf
                elif bt == 'PL':
                    bounds[c][1] = math.inf
                elif bt == 'LI':
                    bounds[c][0] = v
                    integer.add(c)
                elif bt == 'UI':
                    bounds[c][1] = v
                    integer.add(c)
                else:
                    raise ValueError('bound type ' + bt)
            elif section in ('NAME', 'ENDATA'):
                pass
            else:
                raise ValueError('unknown section ' + str(section))
    return dict(rows=rows, row_order=order_rows, obj=obj, cols=cols, col_order=col_order,
                rhs=rhs, ranges=ranges, bounds=bounds, integer=integer, objsense=objsense)


def read_aux(path):
    sec = None
    out = dict(numvars=None, numconstrs=None, vars={}, var_order=[], constrs=[], name=None, mps=None)
    with open(path, encoding='latin-1') as f:
        for raw in f:
            line = raw.strip()
            if not line:
                continue
            if line.startswith('@'):
                sec = line
                continue
            if sec == '@NUMVARS':
                out['numvars'] = int(line)
            elif sec == '@NUMCONSTRS':
                out['numconstrs'] = int(line)
            elif sec == '@VARSBEGIN':
                t = line.split()
                out['vars'][t[0]] = float(t[1])
                out['var_order'].append(t[0])
            elif sec == '@CONSTRSBEGIN':
                out['constrs'].append(line.split()[0])
            elif sec == '@NAME':
                out['name'] = line
            elif sec == '@MPS':
                out['mps'] = line
            else:
                raise ValueError('aux section %s line %s' % (sec, line))
    return out
